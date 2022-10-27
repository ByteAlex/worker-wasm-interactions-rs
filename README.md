# Cloudflare Worker WASM Interactions for Rust 
[![Latest Version]][crates.io]

[Latest Version]: https://img.shields.io/crates/v/worker-wasm-interactions-rs.svg
[crates.io]: https://crates.io/crates/worker-wasm-interactions-rs

Simply build your interaction based discord bot on Cloudflare Workers using Rust.

## Example Code
```rust
use phf::phf_map;
use worker::*;
use worker_wasm_interactions_rs::{CustomIdPattern, Interactions, RouterExt};
use worker_wasm_interactions_rs::twilight_model::guild::PartialMember;
use worker_wasm_interactions_rs::twilight_model::id::Id;

static GENDERS: phf::Map<&'static str, u64> = phf_map! {
    "male" => 1031539478478721064,
    "female" => 1031539718460018758,
    "non-binary" => 1031539776123322418,
};

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}]",
        Date::now().to_string(),
        req.path(),
    );
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    log_request(&req);

    let mut interactions = Interactions::new(env.secret("DISCORD_PUBLIC_KEY")?.to_string(),
                                             env.secret("DISCORD_TOKEN")?.to_string());

    interactions.register_application_command_handler("ping", |context| async move {
        context.followup(true, |builder| {
            builder.content("Pong");
        })
    });

    interactions.register_message_component_handler(CustomIdPattern::starts_with("gender-"), |context| async move {
        if !context.raw.is_guild() {
            return context.followup(true, |builder| {
                builder.content("Not suitable for DM use!");
            });
        }
        let member = context.raw.member.as_ref().expect("Guild Interaction requires member");
        let gender = &context.data.custom_id[7..];
        let content = if let Some(gender_role_id) = GENDERS.get(gender) {
            let guild_id = context.guild_id().expect("Guild Interaction requires guild_id");
            let member_id = context.user_id().expect("User object required");
            if has_role(&member, gender_role_id) {
                context.rest.remove_guild_member_role(&guild_id.get(), &member_id.get(), gender_role_id)
                    .await
                    .map_err(worker_wasm_interactions_rs::util::map_error)?;
                "Removed role"
            } else {
                context.rest.add_guild_member_role(&guild_id.get(), &member_id.get(), gender_role_id)
                    .await
                    .map_err(worker_wasm_interactions_rs::util::map_error)?;
                "Added role"
            }
        } else {
            "Unknown gender role"
        };
        context.followup(true, move |builder| {
            builder.content(content);
        })
    });

    let router = Router::with_data(interactions);

    router
        .get("/", |_, _| Response::ok("Hello from Workers!"))
        .interactions("/interaction")
        .get("/worker-version", |_, ctx| {
            let version = ctx.var("WORKERS_RS_VERSION")?.to_string();
            Response::ok(version)
        })
        .run(req, env)
        .await
}

fn has_role(member: &PartialMember, role_id: &u64) -> bool {
    member.roles.iter().any(|role| role.eq(role_id))
}
```