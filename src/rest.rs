use crate::{MessageBuilder, ToOwnedString};
use reqwest::{Client as HttpClient, Method, Response as HttpResponse};
use serde::de::DeserializeOwned;
use twilight_model::channel::Message;
use twilight_model::channel::message::MessageFlags;
use twilight_model::guild::member::MemberIntermediary;
use worker::*;
use crate::model::MemberEditBuilder;

macro_rules! handle_errors {
    ($req:expr) => {
        match $req {
             Ok(res) => {
                if res.status().is_success() {
                    Ok(())
                } else {
                    Err(Error::from(res.text().await.map_err(|err| Error::from(err.to_string()))?))
                }
            }
            Err(err) => Err(Error::from(err.to_string())),
        }
    };
}

#[derive(Debug, Clone)]
pub struct Client {
    token: String,
    client: HttpClient,
}

#[derive(Debug, Clone)]
pub struct RestInteraction {
    client: HttpClient,
    token: String,
    app_id: u64,
    ephemeral: bool,
}

impl Client {
    pub fn new<S: ToOwnedString>(token: S) -> Self {
        Self {
            token: token.to_owned_string(),
            client: HttpClient::default(),
        }
    }

    pub fn interaction(&self, app_id: u64, interaction_token: String, ephemeral: bool) -> RestInteraction {
        RestInteraction {
            client: self.client.clone(),
            token: interaction_token,
            app_id,
            ephemeral,
        }
    }

    pub async fn get_guild_member(&self, guild_id: &u64, member_id: &u64) -> Result<MemberIntermediary> {
        self.request_json(
            Method::GET,
            format!("https://discord.com/api/guilds/{}/members/{}", guild_id, member_id).as_str(),
        ).await
    }

    pub async fn modify_guild_member<F: FnOnce(&mut MemberEditBuilder) -> ()>(&self, guild_id: &u64, member_id: &u64, builder_fn: F) -> Result<()> {
        let mut builder = MemberEditBuilder::default();
        builder_fn(&mut builder);
        handle_errors!(self.client.request(Method::PATCH, format!("https://discord.com/api/guilds/{}/members/{}", guild_id, member_id))
            .header("Authorization", format!("Bot {}", self.token))
            .json(&builder)
            .send()
            .await)
    }

    pub async fn add_guild_member_role(&self, guild_id: &u64, member_id: &u64, role_id: &u64) -> Result<()> {
        self.request(
            Method::PUT,
            format!("https://discord.com/api/guilds/{}/members/{}/roles/{}", guild_id, member_id, role_id).as_str(),
            Some("Reaction Role invoked"),
        ).await.map(|_| ())
    }


    pub async fn remove_guild_member_role(&self, guild_id: &u64, member_id: &u64, role_id: &u64) -> Result<()> {
        self.request(
            Method::DELETE,
            format!("https://discord.com/api/guilds/{}/members/{}/roles/{}", guild_id, member_id, role_id).as_str(),
            Some("Reaction Role invoked"),
        ).await.map(|_| ())
    }

    pub async fn request_channel_messages(&self, channel_id: &u64,
                                          around: Option<u64>, before: Option<u64>, after: Option<u64>,
                                          limit: Option<u8>) -> Result<Vec<Message>> {
        let mut base_url = format!("https://discord.com/api/channels/{}/messages", channel_id);
        let mut first_query = true;
        if let Some(around) = around {
            first_query = false;
            base_url.push_str("?around=");
            base_url.push_str(around.to_string().as_str());
        } else if let Some(before) = before {
            first_query = false;
            base_url.push_str("?before=");
            base_url.push_str(before.to_string().as_str());
        } else if let Some(after) = after {
            first_query = false;
            base_url.push_str("?after=");
            base_url.push_str(after.to_string().as_str());
        }
        if let Some(limit) = limit {
            if first_query {
                base_url.push_str("?");
            } else {
                base_url.push_str("&");
            }
            base_url.push_str("limit=");
            base_url.push_str(limit.to_string().as_str());
        }
        self.request_json(
            Method::GET,
            base_url.as_str(),
        ).await
    }

    pub async fn delete_message<S: ToOwnedString>(&self, channel_id: &u64, message_id: &u64,
                                                  audit_log_reason: Option<S>) -> Result<()> {
        self.request(
            Method::DELETE,
            format!("https://discord.com/api/channels/{}/messages/{}", channel_id, message_id).as_str(),
            audit_log_reason,
        ).await.map(|_| ())
    }

    pub async fn request_json<T: DeserializeOwned>(&self, method: Method, path: &str) -> Result<T> {
        self.request::<&str>(method, path, None).await?
            .json().await
            .map_err(crate::util::map_error)
    }

    pub async fn request<S: ToOwnedString>(&self, method: Method, path: &str, audit_log_reason: Option<S>) -> Result<HttpResponse> {
        let mut request_builder = self.client.request(method, path)
            .header("Authorization", format!("Bot {}", self.token.as_str()));
        if let Some(reason) = audit_log_reason {
            request_builder = request_builder
                .header("X-Audit-Log-Reason", reason.to_owned_string());
        }
        match request_builder.send().await {
            Ok(res) => {
                if res.status().is_success() {
                    Ok(res)
                } else {
                    Err(Error::from(res.text().await.expect("Failed to get response body")))
                }
            }
            Err(err) => Err(Error::from(err.to_string())),
        }
    }
}

impl RestInteraction {
    pub async fn followup<F: FnOnce(&mut MessageBuilder) -> ()>(&self, message_builder: F) -> Result<Message> {
        let mut builder = MessageBuilder::default();
        message_builder(&mut builder);
        if self.ephemeral {
            if let Some(flags) = builder.flags.as_mut() {
                flags.insert(MessageFlags::EPHEMERAL)
            } else {
                builder.flags = Some(MessageFlags::EPHEMERAL)
            }
        }
        self.client.request(Method::POST, format!("https://discord.com/api/webhooks/{}/{}", self.app_id, self.token))
            .json(&builder)
            .send()
            .await
            .map_err(|err| Error::from(err.to_string()))?
            .json()
            .await
            .map_err(|err| Error::from(err.to_string()))
    }
}