use serde::{Serialize, Deserialize};
use serde_json::{json, Value};

use crate::unid::{keyring::secp256k1::KeyPairSecp256K1, errors::UNiDError};
use crate::unid::runtime::multihash::Multihash;
use crate::unid::runtime::base64_url::Base64Url;
use crate::unid::runtime::base64_url::PaddingType;

pub struct OperationPayload {}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    #[serde(rename = "id")]
    pub id: String,

    #[serde(rename = "type")]
    pub r#type: String,

    #[serde(rename = "serviceEndpoint")]
    pub service_endpoint: String,

    #[serde(rename = "description")]
    pub description: Option<String>,
}
  
#[derive(Debug, Serialize, Deserialize)]
pub struct DidPublicKey {
    #[serde(rename = "id")]
    pub id: String,

    #[serde(rename = "controller")]
    pub controller: String,

    #[serde(rename = "type")]
    pub r#type: String,

    #[serde(rename = "publicKeyJwk")]
    pub public_key_jwk: KeyPairSecp256K1,
}
  
#[derive(Debug, Serialize, Deserialize)]
struct Authentication {
    #[serde(rename = "type")]
    r#type: String,

    #[serde(rename = "publicKey")]
    public_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DIDDocument {
    // TODO: impl parser for mixed type
    // #[serde(rename = "@context")]
    // context: String,

    #[serde(rename = "id")]
    pub id: String,

    #[serde(rename = "publicKey")]
    pub public_key: Option<Vec<DidPublicKey>>,

    #[serde(rename = "service")]
    pub service: Option<Vec<ServiceEndpoint>>,

    // TODO: impl parser for mixed type
    #[serde(rename = "authentication")]
    pub authentication: Option<Vec<DidPublicKey>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublicKeyPayload {
    pub id     : String,
    pub r#type : String,
    pub jwk    : KeyPairSecp256K1,
    pub purpose: Vec<String>,
}

// ACTION: add-public-keys
struct DIDAddPublicKeysPayload {
    id     : String,
    r#type : String,
    jwk    : KeyPairSecp256K1,
    purpose: Vec<String>,
}

struct DIDAddPublicKeysAction {
    action     : String, //'add-public-keys',
    public_keys: Vec<DIDAddPublicKeysPayload>
}

// ACTION: remove-public-keys
struct DIDRemovePublicKeysAction {
    action: String, // 'remove-public-keys',
    ids   : Vec<String>,
}

// ACTION: add-services
struct DIDAddServicesPayload {
}

struct DIDAddServicesAction {
    action  : String, // 'add-services',
    services: Vec<DIDAddServicesPayload>,
}

// ACTION: remove-services
struct DIDRemoveServicesAction {
    action: String, // 'remove-services',
    ids   : Vec<String>,
}

// ACTION: replace
#[derive(Debug, Serialize, Deserialize)]
struct DIDReplacePayload {
    public_keys: Vec<PublicKeyPayload>,
    service_endpoints: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DIDReplaceAction {
    action  : String, // 'replace',
    document: DIDReplacePayload,
}

#[derive(Serialize, Deserialize, Debug)]
struct DIDReplaceDeltaObject {
    patches: Vec<DIDReplaceAction>,
    update_commitment: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DIDReplaceSuffixObject {
    delta_hash: String,
    recovery_commitment: String,
}

// ACTION: ietf-json-patch
struct DIDIetfJsonPatchAction {
    action : String, // 'replace',
    // patches: Vec<>
}

struct DIDResolutionRequest {
    did: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MethodMetadata {
    #[serde(rename = "published")]
    pub published: bool,

    #[serde(rename = "recoveryCommitment")]
    pub recovery_commitment: Option<String>,

    #[serde(rename = "updateCommitment")]
    pub update_commitment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DIDResolutionResponse {
    #[serde(rename = "@context")]
    pub context: String,

    #[serde(rename = "didDocument")]
    pub did_document: DIDDocument,

    #[serde(rename = "methodMetadata")]
    pub method_metadata: MethodMetadata,
}

#[derive(Clone)]
pub struct CommitmentKeys {
    pub recovery: KeyPairSecp256K1,
    pub update  : KeyPairSecp256K1,
}

#[derive(Clone)]
pub struct DIDCreateRequest {
    pub public_keys: Vec<PublicKeyPayload>,
    pub commitment_keys: CommitmentKeys,
    pub service_endpoints: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DIDCreatePayload {
    r#type: String, // 'create',
    delta: String,
    suffix_data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DIDCreateResponse {
    #[serde(rename = "@context")]
    pub context: String,

    #[serde(rename = "didDocument")]
    pub did_document: DIDDocument,

    #[serde(rename = "methodMetadata")]
    pub method_metadata: MethodMetadata,
}

struct DIDUpdateRequest {
    // NOT IMPLEMENTED
}

struct DIDUpdateResponse {
    // NOT IMPLEMENTED
}

struct DIDRecoverRequest {
    // NOT IMPLEMENTED
}

struct DIDRecoverResponse {
    // NOT IMPLEMENTED
}

struct DIDDeactivateRequest {
    // NOT IMPLEMENTED
}

struct DIDDeactivateResponse {
    // NOT IMPLEMENTED
}

impl OperationPayload {
    pub fn did_create_payload(params: &DIDCreateRequest) -> Result<String, UNiDError> {
        let update = json!(&params.commitment_keys.update);
        let update_commitment = match Multihash::canonicalize_then_double_hash_then_encode(&update.to_string().as_bytes()) {
            Ok(v) => v,
            Err(_) => return Err(UNiDError{})
        };

        let recovery = json!(&params.commitment_keys.recovery);
        let recovery_commitment = match Multihash::canonicalize_then_double_hash_then_encode(&recovery.to_string().as_bytes()) {
            Ok(v) => v,
            Err(_) => return Err(UNiDError{})
        };

        let document: DIDReplacePayload = DIDReplacePayload {
            public_keys: params.public_keys.clone(),
            service_endpoints: params.service_endpoints.clone(),
        };
        let patch: DIDReplaceAction = DIDReplaceAction {
            action  : "replace".to_string(),
            document: document,
        };

        let delta = json!(DIDReplaceDeltaObject {
            patches: vec![ patch ],
            update_commitment: update_commitment,
        }).to_string();

        let delta_bytes = delta.as_bytes();
        let delta_hash = Base64Url::encode(
            &Multihash::hash(&delta_bytes), &PaddingType::NoPadding
        );

        let suffix = json!(DIDReplaceSuffixObject {
            delta_hash: delta_hash,
            recovery_commitment: recovery_commitment,
        }).to_string();

        let suffix_bytes = suffix.as_bytes();

        let encoded_delta = Base64Url::encode(&delta_bytes, &PaddingType::NoPadding);
        let encoded_suffix = Base64Url::encode(&suffix_bytes, &PaddingType::NoPadding);

        let payload: DIDCreatePayload = DIDCreatePayload {
            r#type: "create".to_string(),
            delta: encoded_delta,
            suffix_data: encoded_suffix,
        };

        Ok(json!(payload).to_string())
    }
}

#[cfg(test)]
pub mod tests {
    use crate::unid::keyring;

    use super::*;

    #[test]
    pub fn test_did_create_payload() {
        let keyring = match keyring::mnemonic::MnemonicKeyring::create_keyring() {
            Ok(v) => v,
            Err(_) => panic!(),
        };

        let public = match keyring.get_sign_key_pair().to_public_key("key_id", &vec![""]) {
            Ok(v) => v,
            Err(_) => panic!()
        };
        let update = match keyring.get_recovery_key_pair().to_jwk(false) {
            Ok(v) => v,
            Err(_) => panic!()
        };
        let recovery = match keyring.get_update_key_pair().to_jwk(false) {
            Ok(v) => v,
            Err(_) => panic!()
        };

        let result = match OperationPayload::did_create_payload(&DIDCreateRequest {
            public_keys: vec![ public ],
            commitment_keys: CommitmentKeys {
                recovery: recovery,
                update: update,
            },
            service_endpoints: vec!["https://did.getunid.io".to_string()],
        }) {
            Ok(v) => v,
            Err(_) => panic!()
        };

        println!("{}", json!(&result));
    }
}