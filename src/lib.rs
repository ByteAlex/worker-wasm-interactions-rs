pub mod util;
pub mod rest;
pub mod model;

use std::collections::HashMap;
use std::future::Future;
use std::rc::Rc;
use futures::future::LocalBoxFuture;
use twilight_model::application::interaction::{Interaction, InteractionData, InteractionType};
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::application::interaction::message_component::MessageComponentInteractionData;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType};
use twilight_model::id::Id;
use twilight_model::id::marker::{GuildMarker, UserMarker};
use worker::*;
use crate::util::ToOwnedString;
use crate::rest::Client;

pub use twilight_model;
use twilight_model::channel::message::MessageFlags;
use worker::kv::KvStore;
use crate::model::MessageBuilder;

macro_rules! match_as {
    ($obj:expr, $otype:path) => {
        if let $otype(data) = $obj {
            data
        } else {
            panic!("match_as! doesn't match!")
        }
    };
}

pub trait RouterExt {
    fn interactions(self, pattern: &str) -> Self;
}

pub trait GetInteractionData {
    fn get_interactions(&self) -> &Interactions;
}

impl GetInteractionData for Interactions {
    fn get_interactions(&self) -> &Interactions {
        self
    }
}

type InteractionResult = Result<InteractionResponse>;

type InternalCommandHandler = Rc<dyn 'static + Fn(InteractionContext<Box<CommandData>>) -> LocalBoxFuture<'static, InteractionResult>>;
pub type CommandHandler<T> = fn(InteractionContext<Box<CommandData>>) -> T;

type InternalComponentHandler = Rc<dyn 'static + Fn(InteractionContext<MessageComponentInteractionData>) -> LocalBoxFuture<'static, InteractionResult>>;
pub type ComponentHandler<T> = fn(InteractionContext<MessageComponentInteractionData>) -> T;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CustomIdPattern {
    starts_with: Option<String>,
    equals: Option<String>,
}

impl CustomIdPattern {
    pub fn starts_with<S: ToOwnedString>(pattern: S) -> Self {
        Self {
            starts_with: Some(pattern.to_owned_string()),
            equals: None,
        }
    }

    pub fn equals<S: ToOwnedString>(custom_id: S) -> Self {
        Self {
            starts_with: None,
            equals: Some(custom_id.to_owned_string()),
        }
    }

    fn matches(&self, custom_id: &str) -> bool {
        if let Some(pattern) = self.starts_with.as_ref() {
            custom_id.starts_with(pattern)
        } else if let Some(equals) = self.equals.as_ref() {
            custom_id.eq(equals)
        } else {
            false
        }
    }
}

pub struct InteractionContext<D> {
    pub raw: Interaction,
    pub data: D,
    pub rest: Client,
    pub worker_env: Env
}

impl<D> InteractionContext<D> {
    fn create(interaction: Interaction, data: D, token: String, worker_env: Env) -> Self {
        Self {
            raw: interaction,
            data,
            rest: Client::new(token),
            worker_env,
        }
    }

    pub fn guild_id(&self) -> Option<Id<GuildMarker>> {
        self.raw.guild_id
    }

    pub fn user_id(&self) -> Option<Id<UserMarker>> {
        self.raw.author_id()
    }

    pub fn followup<F: FnOnce(&mut MessageBuilder) -> ()>(&self, ephemeral: bool, message_builder: F) -> Result<InteractionResponse> {
        let mut builder = MessageBuilder::default();
        message_builder(&mut builder);
        if ephemeral {
            if let Some(flags) = builder.flags.as_mut() {
                flags.insert(MessageFlags::EPHEMERAL)
            } else {
                builder.flags = Some(MessageFlags::EPHEMERAL)
            }
        }
        Ok(InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData::from(builder))
        })
    }

    // WORKER ENV BINDINGS

    pub fn secret(&self, binding: &str) -> Result<Secret> {
        self.worker_env.secret(binding)
    }

    pub fn var(&self, binding: &str) -> Result<Var> {
        self.worker_env.var(binding)
    }

    pub fn kv(&self, binding: &str) -> Result<KvStore> {
        KvStore::from_this(&self.worker_env, binding).map_err(From::from)
    }

    pub fn durable_object(&self, binding: &str) -> Result<ObjectNamespace> {
        self.worker_env.durable_object(binding)
    }
}

pub struct Interactions {
    public_key: String,
    token: String,
    app_command_handlers: HashMap<&'static str, InternalCommandHandler>,
    msg_component_handlers: HashMap<CustomIdPattern, InternalComponentHandler>,
}

impl Interactions {
    pub fn new(public_key: String, token: String) -> Self {
        Self {
            public_key,
            token,
            app_command_handlers: HashMap::new(),
            msg_component_handlers: HashMap::new(),
        }
    }

    pub fn register_application_command_handler<T: 'static + Future<Output=Result<InteractionResponse>>>(&mut self, command_name: &'static str, handler: CommandHandler<T>) {
        let internal_handler: InternalCommandHandler = Rc::new(move |ctx| Box::pin(handler(ctx)));
        self.app_command_handlers.insert(command_name, internal_handler);
    }

    pub fn register_message_component_handler<T: 'static + Future<Output=Result<InteractionResponse>>>(&mut self, custom_id: CustomIdPattern, handler: ComponentHandler<T>) {
        let internal_handler: InternalComponentHandler = Rc::new(move |ctx| Box::pin(handler(ctx)));
        self.msg_component_handlers.insert(custom_id, internal_handler);
    }

    async fn handle_application_command(&self, context: InteractionContext<Box<CommandData>>) -> Result<Response> {
        if let Some(handler) = self.app_command_handlers.get(context.data.name.as_str()) {
            let result: InteractionResult = (handler)(context).await;
            match result {
                Ok(response) => Response::from_json(&response),
                Err(err) => Response::from_json(&util::error_message(format!("An error occurred: {}", err.to_string())))
            }
        } else {
            Response::from_json(&util::error_message("This command is not registered".to_string()))
        }
    }

    async fn handle_message_component(&self, context: InteractionContext<MessageComponentInteractionData>) -> Result<Response> {
        if let Some(handler) = self.msg_component_handlers.iter()
            .find(|(pattern, _)| pattern.matches(context.data.custom_id.as_str()))
            .map(|(_, handler)| handler) {
            let result: InteractionResult = (handler)(context).await;
            match result {
                Ok(response) => Response::from_json(&response),
                Err(err) => Response::from_json(&util::error_message(format!("An error occurred: {}", err.to_string())))
            }
        } else {
            Response::from_json(&util::error_message("This message component is not registered".to_string()))
        }
    }
}

impl<'a, D: GetInteractionData + 'a> RouterExt for Router<'a, D> {
    fn interactions(self, pattern: &str) -> Self {
        self.post_async(pattern, |mut req, ctx| async move {
            let body = req.bytes().await?;
            let interactions_lib = ctx.data.get_interactions();
            if !util::validate_headers(&req, body.as_slice(), interactions_lib.public_key.as_str())? {
                return Response::error("Invalid token", 401);
            }
            let interaction: Interaction = serde_json::from_slice(body.as_slice())?;

            match interaction.kind {
                InteractionType::Ping => Response::from_json(&InteractionResponse {
                    kind: InteractionResponseType::Pong,
                    data: None,
                }),
                InteractionType::ApplicationCommand => {
                    let command = match_as!(interaction.data.clone().expect("Missing data"), InteractionData::ApplicationCommand);
                    let context = InteractionContext::create(interaction, command, interactions_lib.token.clone(), ctx.env);
                    interactions_lib.handle_application_command(context).await
                }
                InteractionType::MessageComponent => {
                    let component = match_as!(interaction.data.clone().expect("Missing data"), InteractionData::MessageComponent);
                    let context = InteractionContext::create(interaction, component, interactions_lib.token.clone(), ctx.env);
                    interactions_lib.handle_message_component(context).await
                }
                _ => Response::error("Missing implementation", 400)
            }
        })
    }
}