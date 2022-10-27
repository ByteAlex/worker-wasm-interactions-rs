use ed25519_dalek::{PUBLIC_KEY_LENGTH, PublicKey, Signature, SIGNATURE_LENGTH, Verifier};
use twilight_model::channel::message::MessageFlags;
use twilight_model::http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType};
use worker::*;

pub trait ToOwnedString {
    fn to_owned_string(self) -> String;
}

impl ToOwnedString for String {
    fn to_owned_string(self) -> String {
        self
    }
}

impl ToOwnedString for &str {
    fn to_owned_string(self) -> String {
        self.to_string()
    }
}

pub fn map_error<T: ToString>(error: T) -> Error {
    Error::RustError(error.to_string())
}

pub fn validate_headers<S: AsRef<str>>(req: &Request, body: &[u8], public_key: S) -> Result<bool> {
    let sig = req.headers().get("x-signature-ed25519")?;
    let timestamp = req.headers().get("x-signature-timestamp")?;
    if sig.is_none() || timestamp.is_none() {
        return Ok(false);
    }
    let sig = sig.unwrap();
    let timestamp = timestamp.unwrap();

    let public_key_hex = hex::decode(public_key.as_ref())
        .map_err(|err| Error::from(err.to_string()))?;
    let signature_hex = hex::decode(sig)
        .map_err(|err| Error::from(err.to_string()))?;
    let public_key = PublicKey::from_bytes(&public_key_hex.as_slice()[..PUBLIC_KEY_LENGTH]).unwrap();
    let signature = Signature::from_bytes(&signature_hex.as_slice()[..SIGNATURE_LENGTH]).unwrap();

    let mut full_body = timestamp.into_bytes();
    full_body.extend_from_slice(body);

    if let Err(err) = public_key.verify(full_body.as_slice(), &signature) {
        console_log!("Received invalid signature: {}", err);
        Ok(false)
    } else {
        Ok(true)
    }
}

pub(crate) fn error_message(message: String) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            allowed_mentions: None,
            attachments: None,
            choices: None,
            components: None,
            content: Some(message),
            custom_id: None,
            embeds: None,
            flags: Some(MessageFlags::EPHEMERAL),
            title: None,
            tts: None,
        }),
    }
}