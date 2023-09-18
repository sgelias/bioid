use super::{account::Account, email::Email};

use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use chrono::{DateTime, Local};
use clean_base::dtos::Parent;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PasswordHash {
    pub hash: String,
    pub salt: String,
}

impl PasswordHash {
    pub fn new(hash: String, salt: String) -> Self {
        Self { hash, salt }
    }

    pub async fn hash_user_password(password: &[u8]) -> Self {
        let salt = SaltString::generate(&mut OsRng);

        Self {
            salt: salt.to_string(),
            hash: Argon2::default()
                .hash_password(password, &salt)
                .expect("Unable to hash password.")
                .to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Provider {
    External,
    Internal(PasswordHash),
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Option<Uuid>,

    pub username: String,
    pub email: Email,
    pub provider: Option<Provider>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub is_active: bool,
    pub created: DateTime<Local>,
    pub updated: Option<DateTime<Local>>,
    pub account: Option<Parent<Account, Uuid>>,
}
