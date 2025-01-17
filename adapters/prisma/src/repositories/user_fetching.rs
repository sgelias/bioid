use crate::{
    prisma::{user as user_model, QueryMode},
    repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::Local;
use myc_core::domain::{
    dtos::{
        email::Email,
        native_error_codes::NativeErrorCodes,
        user::{MultiFactorAuthentication, PasswordHash, Provider, User},
    },
    entities::UserFetching,
};
use mycelium_base::{
    dtos::Parent,
    entities::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use prisma_client_rust::and;
use serde_json::from_value;
use shaku::Component;
use std::process::id as process_id;
use tracing::error;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = UserFetching)]
pub struct UserFetchingSqlDbRepository {}

#[async_trait]
impl UserFetching for UserFetchingSqlDbRepository {
    #[tracing::instrument(name = "get_user_by_email", skip_all)]
    async fn get_user_by_email(
        &self,
        email: Email,
    ) -> Result<FetchResponseKind<User, String>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return fetching_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Get the user
        // ? -------------------------------------------------------------------

        match client
            .user()
            .find_first(vec![and![
                user_model::email::mode(QueryMode::Insensitive),
                user_model::email::equals(email.email())
            ]])
            .include(user_model::include!({ provider }))
            .exec()
            .await
        {
            Err(err) => {
                return fetching_err(format!(
                    "Unexpected error on parse user email: {:?}",
                    err
                ))
                .as_error()
            }
            Ok(res) => match res {
                None => Ok(FetchResponseKind::NotFound(None)),
                Some(record) => {
                    if record.provider.is_none() {
                        return fetching_err(String::from(
                            "User has not a email provider",
                        ))
                        .as_error();
                    }

                    let record_provider = &record.provider.unwrap();
                    let record_password_hash = &record_provider.password_hash;
                    let record_provider_name = &record_provider.name;

                    let provider = {
                        if record_password_hash.is_some() {
                            Provider::Internal(PasswordHash::new_from_hash(
                                record_password_hash.clone().unwrap(),
                            ))
                        } else if record_provider_name.is_some() {
                            Provider::External(
                                record_provider_name.clone().unwrap(),
                            )
                        } else {
                            return fetching_err(String::from(
                                "Unexpected error on parse user email: {:?}",
                            ))
                            .as_error();
                        }
                    };

                    let mut user = User::new(
                        Some(Uuid::parse_str(&record.id).unwrap()),
                        record.username,
                        Email::from_string(record.email).unwrap(),
                        Some(record.first_name),
                        Some(record.last_name),
                        record.is_active,
                        record.created.into(),
                        match record.updated {
                            None => None,
                            Some(date) => Some(date.with_timezone(&Local)),
                        },
                        match &record.account_id {
                            Some(id) => {
                                Some(Parent::Id(Uuid::parse_str(id).unwrap()))
                            }
                            None => None,
                        },
                        Some(provider),
                    )
                    .with_principal(record.is_principal);

                    if let Some(mfa) = record.mfa {
                        let mut mfa: MultiFactorAuthentication =
                            match from_value(mfa) {
                                Ok(res) => res,
                                Err(err) => {
                                    error!("Unexpected error on fetch user mfa: {:?}", err);

                                    return fetching_err(
                                        "Unexpected error on check user mfa",
                                    )
                                    .as_error();
                                }
                            };

                        mfa.redact_secrets();
                        user = user.with_mfa(mfa);
                    }

                    Ok(FetchResponseKind::Found(user))
                }
            },
        }
    }

    #[tracing::instrument(name = "get_user_by_id", skip_all)]
    async fn get_user_by_id(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<User, String>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return fetching_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Get the user
        // ? -------------------------------------------------------------------

        match client
            .user()
            .find_unique(user_model::id::equals(id.to_string()))
            .include(user_model::include!({ provider }))
            .exec()
            .await
        {
            Err(err) => {
                return fetching_err(format!(
                    "Unexpected error on parse user email: {:?}",
                    err
                ))
                .as_error()
            }
            Ok(res) => match res {
                None => Ok(FetchResponseKind::NotFound(None)),
                Some(record) => {
                    if record.provider.is_none() {
                        return fetching_err(String::from(
                            "Unexpected error on parse user: {:?}",
                        ))
                        .as_error();
                    }

                    let record_provider = &record.provider.unwrap();
                    let record_password_hash = &record_provider.password_hash;
                    let record_provider_name = &record_provider.name;

                    let provider = {
                        if record_password_hash.is_some() {
                            Provider::Internal(PasswordHash::new_from_hash(
                                record_password_hash.clone().unwrap(),
                            ))
                        } else if record_provider_name.is_some() {
                            Provider::External(
                                record_provider_name.clone().unwrap(),
                            )
                        } else {
                            return fetching_err(String::from(
                                "Unexpected error on parse user email: {:?}",
                            ))
                            .as_error();
                        }
                    };

                    let mut user = User::new(
                        Some(Uuid::parse_str(&record.id).unwrap()),
                        record.username,
                        Email::from_string(record.email).unwrap(),
                        Some(record.first_name),
                        Some(record.last_name),
                        record.is_active,
                        record.created.into(),
                        match record.updated {
                            None => None,
                            Some(date) => Some(date.with_timezone(&Local)),
                        },
                        match &record.account_id {
                            Some(id) => {
                                Some(Parent::Id(Uuid::parse_str(id).unwrap()))
                            }
                            None => None,
                        },
                        Some(provider),
                    )
                    .with_principal(record.is_principal);

                    if let Some(mfa) = record.mfa {
                        let mut mfa: MultiFactorAuthentication =
                            match from_value(mfa) {
                                Ok(res) => res,
                                Err(err) => {
                                    error!("Unexpected error on fetch user mfa: {:?}", err);

                                    return fetching_err(
                                        "Unexpected error on check user mfa",
                                    )
                                    .as_error();
                                }
                            };

                        mfa.redact_secrets();
                        user = user.with_mfa(mfa);
                    }

                    Ok(FetchResponseKind::Found(user))
                }
            },
        }
    }

    #[tracing::instrument(name = "get_not_redacted_user_by_email", skip_all)]
    async fn get_not_redacted_user_by_email(
        &self,
        email: Email,
    ) -> Result<FetchResponseKind<User, String>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return fetching_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Get the user
        // ? -------------------------------------------------------------------

        match client
            .user()
            .find_first(vec![and![
                user_model::email::mode(QueryMode::Insensitive),
                user_model::email::equals(email.email())
            ]])
            .include(user_model::include!({ provider }))
            .exec()
            .await
        {
            Err(err) => {
                return fetching_err(format!(
                    "Unexpected error on parse user email: {:?}",
                    err
                ))
                .as_error()
            }
            Ok(res) => {
                match res {
                    None => Ok(FetchResponseKind::NotFound(None)),
                    Some(record) => {
                        if record.provider.is_none() {
                            return fetching_err(String::from(
                                "Unexpected error on parse user: {:?}",
                            ))
                            .as_error();
                        }

                        let record_provider = &record.provider.unwrap();
                        let record_password_hash =
                            &record_provider.password_hash;
                        let record_provider_name = &record_provider.name;

                        let provider = {
                            if record_password_hash.is_some() {
                                Provider::Internal(PasswordHash::new_from_hash(
                                    record_password_hash.clone().unwrap(),
                                ))
                            } else if record_provider_name.is_some() {
                                Provider::External(
                                    record_provider_name.clone().unwrap(),
                                )
                            } else {
                                return fetching_err(String::from(
                                "Unexpected error on parse user email: {:?}",
                            ))
                            .as_error();
                            }
                        };

                        let mut user = User::new(
                            Some(Uuid::parse_str(&record.id).unwrap()),
                            record.username,
                            Email::from_string(record.email).unwrap(),
                            Some(record.first_name),
                            Some(record.last_name),
                            record.is_active,
                            record.created.into(),
                            match record.updated {
                                None => None,
                                Some(date) => Some(date.with_timezone(&Local)),
                            },
                            match &record.account_id {
                                Some(id) => Some(Parent::Id(
                                    Uuid::parse_str(id).unwrap(),
                                )),
                                None => None,
                            },
                            Some(provider),
                        )
                        .with_principal(record.is_principal);

                        user = if let Some(mfa) = record.mfa {
                            let mfa = match from_value(mfa) {
                                Ok(res) => res,
                                Err(err) => {
                                    error!("Unexpected error on fetch user mfa: {:?}", err);

                                    return fetching_err(
                                        "Unexpected error on check user mfa",
                                    )
                                    .as_error();
                                }
                            };

                            user.with_mfa(mfa);
                            user
                        } else {
                            user
                        };

                        Ok(FetchResponseKind::Found(user))
                    }
                }
            }
        }
    }
}
