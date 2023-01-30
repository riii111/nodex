use arrayref::array_ref;
use serde_json::{Value};
use didcomm_rs::{Message, crypto::{SignatureAlgorithm, CryptoAlgorithm}, AttachmentBuilder, AttachmentDataBuilder};
use x25519_dalek::{PublicKey, StaticSecret};
use cuid;
use crate::{unid::{errors::UNiDError, keyring::{self}, runtime::{self, base64_url::{self, PaddingType}}}};
use super::{types::VerifiedContainer, did_vc::DIDVCService};

pub struct DIDCommEncryptedService {}

impl DIDCommEncryptedService {
    pub async fn generate(to_did: &str, message: &Value, metadata: Option<&Value>) -> Result<Value, UNiDError> {
        let service = crate::services::unid::UNiD::new();

        // NOTE: recipient from
        let my_keyring = match keyring::mnemonic::MnemonicKeyring::load_keyring() {
            Ok(v) => v,
            Err(_) => return Err(UNiDError{})
        };
        let my_did = match my_keyring.get_identifier() {
            Ok(v) => v,
            Err(_) => return Err(UNiDError{})
        };

        // NOTE: recipient to
        let did_document = match service.find_identifier(&to_did).await {
            Ok(v) => v,
            Err(_) => return Err(UNiDError{}),
        };
        let public_keys = match did_document.did_document.public_key {
            Some(v) => v,
            None => return Err(UNiDError{}),
        };

        // FIXME: workaround
        if public_keys.len() != 1 {
            return Err(UNiDError{})
        }

        let public_key = public_keys[0].clone();

        let other_key = match keyring::secp256k1::Secp256k1::from_jwk(&public_key.public_key_jwk) {
            Ok(v) => v,
            Err(_) => return Err(UNiDError{}),
        };

        // NOTE: ecdh
        let shared_key = match runtime::secp256k1::Secp256k1::ecdh(
            &my_keyring.get_sign_key_pair().get_secret_key(),
            &other_key.get_public_key(),
        ) {
            Ok(v) => v,
            Err(_) => return Err(UNiDError{})
        };

        let sk = StaticSecret::from(array_ref!(shared_key, 0, 32).to_owned());
        let pk = PublicKey::from(&sk);

        // NOTE: message
        let body = match DIDVCService::generate(&message) {
            Ok(v) => v,
            Err(_) => return Err(UNiDError{}),
        };

        let mut message = Message::new()
            .from(&my_did)
            .to(&[ &to_did ])
            .body(&body.to_string());

        // NOTE: Has attachment
        if let Some(value) = metadata {
            let id = match cuid::cuid() {
                Ok(v) => v,
                _ => return Err(UNiDError{}),
            };

            // let media_type = "application/json";
            let data = AttachmentDataBuilder::new()
                .with_link("https://did.getunid.io")
                .with_json(&value.to_string());

            message.apeend_attachment(
                AttachmentBuilder::new(true)
                .with_id(&id)
                .with_format("metadata")
                .with_data(data)
            )
        }

        match message.clone()
            .as_jwe(&CryptoAlgorithm::XC20P, Some(pk.as_bytes().to_vec()))
            .seal_signed(
                &sk.to_bytes().to_vec(),
                Some(vec![ Some(pk.as_bytes().to_vec()) ]),
                SignatureAlgorithm::Es256k,
                &my_keyring.get_sign_key_pair().get_secret_key()
            ) {
                Ok(v) => {
                    match serde_json::from_str::<Value>(&v) {
                        Ok(v) => Ok(v),
                        Err(_) => return Err(UNiDError{}),
                    }
                },
                Err(_) => return Err(UNiDError{})
            }
    }

    pub async fn verify(message: &Value) -> Result<VerifiedContainer, UNiDError> {
        let service = crate::services::unid::UNiD::new();

        // NOTE: recipient to
        let my_keyring = match keyring::mnemonic::MnemonicKeyring::load_keyring() {
            Ok(v) => v,
            Err(_) => return Err(UNiDError{})
        };

        // NOTE: recipient from
        let protected = match message.get("protected") {
            Some(v) => {
                match v.as_str() {
                    Some(v) => v.to_string(),
                    None => return Err(UNiDError{}),
                }
            },
            None => return Err(UNiDError{})
        };

        let decoded = match base64_url::Base64Url::decode_as_string(&protected, &PaddingType::NoPadding) {
            Ok(v) => {
                match serde_json::from_str::<Value>(&v) {
                    Ok(v) => v,
                    Err(_) => return Err(UNiDError{}),
                }
            },
            Err(_) => return Err(UNiDError{}),
        };

        let other_did = match decoded.get("skid") {
            Some(v) => {
                match v.as_str() {
                    Some(v) => v.to_string(),
                    None => return Err(UNiDError{}),
                }
            },
            None => return Err(UNiDError{}),
        };

        let did_document = match service.find_identifier(&other_did).await {
            Ok(v) => v,
            Err(_) => return Err(UNiDError{}),
        };

        let public_keys = match did_document.did_document.public_key {
            Some(v) => v,
            None => return Err(UNiDError{}),
        };

        // FIXME: workaround
        if public_keys.len() != 1 {
            return Err(UNiDError{})
        }

        let public_key = public_keys[0].clone();

        let other_key = match keyring::secp256k1::Secp256k1::from_jwk(&public_key.public_key_jwk) {
            Ok(v) => v,
            Err(_) => return Err(UNiDError{}),
        };

        // NOTE: ecdh
        let shared_key = match runtime::secp256k1::Secp256k1::ecdh(
            &my_keyring.get_sign_key_pair().get_secret_key(),
            &other_key.get_public_key(),
        ) {
            Ok(v) => v,
            Err(_) => return Err(UNiDError{})
        };

        let sk = StaticSecret::from(array_ref!(shared_key, 0, 32).to_owned());
        let pk = PublicKey::from(&sk);

        let message = match Message::receive(
            &message.to_string(),
            Some(&sk.to_bytes().to_vec()),
            Some(pk.as_bytes().to_vec()),
            Some(&other_key.get_public_key()),
        ) {
            Ok(v) => v,
            Err(_) => return Err(UNiDError{}),
        };

        let metadata = message
            .get_attachments()
            .find(|item| {
                match item.format.clone() {
                    Some(value) => value == "metadata",
                    None => false
                }
            });

        let body = match message.clone().get_body() {
            Ok(v) => {
                match serde_json::from_str::<Value>(&v) {
                    Ok(v) => v,
                    Err(_) => return Err(UNiDError{}),
                }
            },
            Err(_) => return Err(UNiDError{}),
        };

        match metadata {
            Some(metadata) => {
                match metadata.data.json.clone() {
                    Some(json) => {
                        match serde_json::from_str::<Value>(&json) {
                            Ok(metadata) => {
                                Ok(VerifiedContainer {
                                    message: body,
                                    metadata: Some(metadata),
                                })
                            },
                            _ => Err(UNiDError {})
                        }
                    },
                    _ => Err(UNiDError {})
                }
            },
            None => {
                Ok(VerifiedContainer {
                    message: body,
                    metadata: None,
                })
            }
        }
    }
}