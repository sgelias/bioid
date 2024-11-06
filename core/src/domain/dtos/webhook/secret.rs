use crate::models::AccountLifeCycle;

use arrayref::array_ref;
use base64::{engine::general_purpose, Engine};
use mycelium_base::utils::errors::{dto_err, MappedErrors};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use tracing::error;
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum WebHookSecret {
    /// Authentication header
    ///
    /// The secret is passed as an authentication header.
    ///
    #[serde(rename_all = "camelCase")]
    AuthorizationHeader {
        /// The header name
        ///
        /// The name of the header. For example, if the name is `Authorization`,
        /// the header will be `Authorization Bear: <token value>`. The default
        /// value is `Authorization`.
        ///
        #[serde(default = "default_authorization_value")]
        name: Option<String>,

        /// The header prefix
        ///
        /// If present the prefix is added to the header. For example, if the
        /// prefix is `Bearer`, the header will be `Authorization Bearer: <token
        /// value>`.
        ///
        prefix: Option<String>,

        /// The header token
        ///
        /// The token is the value of the header. For example, if the token is
        /// `1234`, the header will be `Authorization Bearer: 123
        ///
        token: String,
    },

    #[serde(rename_all = "camelCase")]
    QueryParameter {
        /// The query parameter name
        ///
        /// The name of the query parameter. For example, if the name is `token`,
        /// the query parameter will be `?token=<token value>`.
        ///
        name: String,

        /// The query parameter value
        ///
        /// The value of the query parameter. For example, if the value is `1234`,
        /// the query parameter will be `?token=1234`.
        ///
        token: String,
    },
}

fn default_authorization_value() -> Option<String> {
    Some("Authorization".to_string())
}

impl WebHookSecret {
    #[tracing::instrument(name = "encrypt_secret", skip_all)]
    pub(crate) fn encrypt_me(
        &self,
        config: AccountLifeCycle,
    ) -> Result<Self, MappedErrors> {
        let encryption_key = config.get_secret()?;

        let unbound_key =
            match UnboundKey::new(&AES_256_GCM, encryption_key.as_bytes()) {
                Ok(key) => key,
                Err(err) => {
                    error!("Failed to create encryption key: {:?}", err);

                    return dto_err("Failed to create encryption key")
                        .as_error();
                }
            };

        let key = LessSafeKey::new(unbound_key);

        // Generate nonce
        let rand = SystemRandom::new();
        let mut nonce_bytes = [0u8; 12];

        match rand.fill(&mut nonce_bytes) {
            Ok(_) => (),
            Err(err) => {
                error!("Failed to generate nonce: {:?}", err);

                return dto_err("Failed to generate nonce").as_error();
            }
        };

        let nonce = Nonce::assume_unique_for_key(nonce_bytes);

        // Crypt secret
        //let mut in_out = secret.as_bytes().to_vec();
        let mut in_out = (match self {
            Self::AuthorizationHeader { token, .. } => token,
            Self::QueryParameter { token, .. } => token,
        })
        .as_bytes()
        .to_vec();

        match key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out) {
            Ok(_) => (),
            Err(err) => {
                error!("Failed to encrypt secret: {:?}", err);

                return dto_err("Failed to encrypt secret").as_error();
            }
        };

        // Concatenate nonce + ciphertext to store
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&in_out);

        let encrypted_decoded_token = general_purpose::STANDARD.encode(result);

        let self_encrypted = match self {
            Self::AuthorizationHeader { name, prefix, .. } => {
                Self::AuthorizationHeader {
                    token: encrypted_decoded_token.to_owned(),
                    name: name.to_owned(),
                    prefix: prefix.to_owned(),
                }
            }
            Self::QueryParameter { name, .. } => Self::QueryParameter {
                token: encrypted_decoded_token.to_owned(),
                name: name.to_owned(),
            },
        };

        Ok(self_encrypted)
    }

    #[tracing::instrument(name = "decrypt_secret", skip_all)]
    pub(crate) fn decrypt_me(
        &self,
        config: AccountLifeCycle,
    ) -> Result<Self, MappedErrors> {
        let encryption_key = config.get_secret()?;

        let encrypted = (match self {
            Self::AuthorizationHeader { token, .. } => token,
            Self::QueryParameter { token, .. } => token,
        })
        .as_bytes()
        .to_vec();

        let encrypted = match general_purpose::STANDARD.decode(encrypted) {
            Ok(data) => data,
            Err(err) => {
                error!("Failed to decode encrypted secret: {:?}", err);

                return dto_err("Failed to decode encrypted secret").as_error();
            }
        };

        let (nonce_bytes, ciphertext) = encrypted.split_at(12);

        let unbound_key =
            match UnboundKey::new(&AES_256_GCM, encryption_key.as_bytes()) {
                Ok(key) => key,
                Err(err) => {
                    error!("Failed to create encryption key: {:?}", err);

                    return dto_err("Failed to create encryption key")
                        .as_error();
                }
            };

        let key = LessSafeKey::new(unbound_key);
        let nonce =
            Nonce::assume_unique_for_key(*array_ref!(nonce_bytes, 0, 12));

        let mut in_out = ciphertext.to_vec();

        match key.open_in_place(nonce, Aad::empty(), &mut in_out) {
            Ok(_) => (),
            Err(err) => {
                error!("Failed to decrypt secret: {:?}", err);

                return dto_err("Failed to decrypt secret").as_error();
            }
        };

        let response = match String::from_utf8(in_out) {
            Ok(response) => response,
            Err(err) => {
                error!(
                    "Failed to convert decrypted secret to string: {:?}",
                    err
                );

                return dto_err("Failed to convert decrypted secret to string")
                    .as_error();
            }
        };

        let self_decrypted = match self {
            Self::AuthorizationHeader { name, prefix, .. } => {
                Self::AuthorizationHeader {
                    token: response.to_owned(),
                    name: name.to_owned(),
                    prefix: prefix.to_owned(),
                }
            }
            Self::QueryParameter { name, .. } => Self::QueryParameter {
                token: response.to_owned(),
                name: name.to_owned(),
            },
        };

        Ok(self_decrypted)
    }
}
