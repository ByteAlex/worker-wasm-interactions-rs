use serde::{Serialize, Deserialize};
use serde_with::skip_serializing_none;
use twilight_model::application::command::CommandOptionChoice;
use twilight_model::application::component::Component;
use twilight_model::channel::embed::Embed;
use twilight_model::channel::message::{AllowedMentions, MessageFlags};
use twilight_model::http::attachment::Attachment;
use twilight_model::http::interaction::InteractionResponseData;
use twilight_model::id::Id;
use twilight_model::id::marker::{ChannelMarker, RoleMarker};
use crate::ToOwnedString;

#[skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct MessageBuilder {
    pub allowed_mentions: Option<AllowedMentions>,
    pub attachments: Option<Vec<Attachment>>,
    pub choices: Option<Vec<CommandOptionChoice>>,
    pub components: Option<Vec<Component>>,
    pub content: Option<String>,
    pub custom_id: Option<String>,
    pub embeds: Option<Vec<Embed>>,
    pub flags: Option<MessageFlags>,
    pub title: Option<String>,
    pub tts: Option<bool>,
}

impl MessageBuilder {
    pub fn content<S: ToOwnedString>(&mut self, content: S) -> &mut Self {
        self.content = Some(content.to_owned_string());
        self
    }

    pub fn custom_id<S: ToOwnedString>(&mut self, custom_id: S) -> &mut Self {
        self.custom_id = Some(custom_id.to_owned_string());
        self
    }

    pub fn title<S: ToOwnedString>(&mut self, title: S) -> &mut Self {
        self.title = Some(title.to_owned_string());
        self
    }
}

#[skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct MemberEditBuilder {
    pub nick: Option<String>,
    pub roles: Option<Vec<Id<RoleMarker>>>,
    pub mute: Option<bool>,
    pub deaf: Option<bool>,
    pub channel_id: Option<Id<ChannelMarker>>,
    pub communication_disabled_until: Option<String>,
}

impl MemberEditBuilder {
    pub fn nick<S: ToOwnedString>(&mut self, nick: S) -> &mut Self {
        self.nick = Some(nick.to_owned_string());
        self
    }

    pub fn roles(&mut self, roles: Vec<u64>) -> &mut Self {
        self.roles = Some(roles.into_iter()
            .map(|id| Id::new(id))
            .collect());
        self
    }

    pub fn roles_marker(&mut self, roles: Vec<Id<RoleMarker>>) -> &mut Self {
        self.roles = Some(roles);
        self
    }
}

impl From<MessageBuilder> for InteractionResponseData {
    fn from(builder: MessageBuilder) -> Self {
        Self {
            allowed_mentions: builder.allowed_mentions,
            attachments: builder.attachments,
            choices: builder.choices,
            components: builder.components,
            content: builder.content,
            custom_id: builder.custom_id,
            embeds: builder.embeds,
            flags: builder.flags,
            title: builder.title,
            tts: builder.tts,
        }
    }
}