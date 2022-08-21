use serde::{Serialize, Deserialize};
use serde_json::Value;

use crate::unid::cipher::credential_signer::Proof;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Issuer {
    #[serde(rename = "id")]
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct CredentialSubject {
    #[serde(rename = "id")]
    pub id: String,

    #[serde(rename = "container")]
    pub container: Value,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct GeneralVcDataModel {
    #[serde(rename = "id")]
    pub id: String,

    #[serde(rename = "issuer")]
    pub issuer: Issuer,

    #[serde(rename = "issuanceDate")]
    pub issuance_date: String,

    #[serde(rename = "expirationDate")]
    pub expiration_date: Option<String>,

    #[serde(rename = "@context")]
    pub context: Vec<String>,

    #[serde(rename = "type")]
    pub r#type: Vec<String>,

    #[serde(rename = "credentialSubject")]
    pub credential_subject: CredentialSubject,

    #[serde(rename = "proof", skip_serializing_if = "Option::is_none")]
    pub proof: Option<Proof>,
}