# Cloudflare Worker WASM Interactions for Rust

Simply build your interaction based discord bot on Cloudflare Workers using Rust.

## Example Code

Please be aware that you should not make a reaction-roles with the `roleId` in the `customId` field, as this can be 
easily manipulated. For simplicity of this example, we ignore this vulnerability here.

```rust
use std::str::FromStr;
use twilight_model::id::Id;
use worker::*;
use crate::{CustomIdPattern, Interactions, util};
use crate::RouterExt;

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let mut interactions = Interactions::new(env.secret("DISCORD_PUBLIC_KEY")?.to_string(),
                                             env.secret("DISCORD_TOKEN")?.to_string());

    interactions.register_application_command_handler("ping", |context| async move {
        context.followup(true, |builder| {
            builder.content("Pong");
        })
    });

    let pattern = CustomIdPattern::starts_with("rr-".to_string());
    interactions.register_message_component_handler(pattern, |context| async move {
        let role_id = context.data.custom_id.as_str()[3..].to_string();
        context.rest.add_guild_member_role(
            context.guild_id().unwrap(),
            context.user_id().unwrap(),
            Id::from_str(role_id.as_str()).unwrap(),
        ).exec().await.map_err(util::map_error)?;

        context.followup(true, |builder| {
            builder.content("Done!");
        })
    });

    let router = Router::with_data(interactions);

    router
        .get("/", |_, _| Response::ok("Hello from Workers!"))
        .interactions("/interaction")
        .run(req, env)
        .await
}

```