use crate::{
    prisma::account as account_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::{DateTime, Local};
use myc_core::domain::{
    dtos::{
        account::{Account, AccountMeta, AccountMetaKey, VerboseStatus},
        account_type::AccountType,
        email::Email,
        native_error_codes::NativeErrorCodes,
        tag::Tag,
        user::User,
    },
    entities::AccountUpdating,
};
use mycelium_base::{
    dtos::{Children, Parent},
    entities::UpdatingResponseKind,
    utils::errors::{updating_err, MappedErrors},
};
use prisma_client_rust::{
    prisma_errors::query_engine::RecordNotFound, QueryError,
};
use serde_json::{from_value, to_value};
use shaku::Component;
use std::{collections::HashMap, process::id as process_id};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountUpdating)]
pub struct AccountUpdatingSqlDbRepository {}

#[async_trait]
impl AccountUpdating for AccountUpdatingSqlDbRepository {
    async fn update(
        &self,
        account: Account,
    ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return updating_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Try to update record
        // ? -------------------------------------------------------------------

        let account_id = match account.id {
            None => {
                return updating_err(String::from(
                    "Unable to update account. Invalid record ID",
                ))
                .with_exp_true()
                .as_error()
            }
            Some(res) => res,
        };

        let response = client
            .account()
            .update(
                account_model::id::equals(account_id.to_string()),
                vec![
                    account_model::name::set(account.name),
                    account_model::slug::set(account.slug),
                    account_model::is_active::set(account.is_active),
                    account_model::is_checked::set(account.is_checked),
                    account_model::is_archived::set(account.is_archived),
                    account_model::is_default::set(account.is_default),
                    account_model::account_type::set(
                        to_value(account.account_type).unwrap(),
                    ),
                ],
            )
            .include(account_model::include!({
                owners
                tags: select {
                    id
                    value
                    meta
                }
            }))
            .exec()
            .await;

        match response {
            Ok(record) => {
                let id = Uuid::parse_str(&record.id).unwrap();

                Ok(UpdatingResponseKind::Updated(Account {
                    id: Some(id.to_owned()),
                    name: record.name,
                    slug: record.slug,
                    tags: match record.tags.len() {
                        0 => None,
                        _ => Some(
                            record
                                .tags
                                .to_owned()
                                .into_iter()
                                .map(|i| Tag {
                                    id: Uuid::parse_str(&i.id).unwrap(),
                                    value: i.value,
                                    meta: match i.meta {
                                        None => None,
                                        Some(meta) => {
                                            Some(from_value(meta).unwrap())
                                        }
                                    },
                                })
                                .collect::<Vec<Tag>>(),
                        ),
                    },
                    is_active: record.is_active,
                    is_checked: record.is_checked,
                    is_archived: record.is_archived,
                    verbose_status: Some(VerboseStatus::from_flags(
                        record.is_active,
                        record.is_checked,
                        record.is_archived,
                    )),
                    is_default: record.is_default,
                    owners: Children::Records(
                        record
                            .owners
                            .into_iter()
                            .map(|owner| {
                                User::new(
                                    Some(Uuid::parse_str(&owner.id).unwrap()),
                                    owner.username,
                                    Email::from_string(owner.email).unwrap(),
                                    Some(owner.first_name),
                                    Some(owner.last_name),
                                    owner.is_active,
                                    owner.created.into(),
                                    match owner.updated {
                                        None => None,
                                        Some(date) => {
                                            Some(date.with_timezone(&Local))
                                        }
                                    },
                                    Some(Parent::Id(id)),
                                    None,
                                )
                                .with_principal(owner.is_principal)
                            })
                            .collect::<Vec<User>>(),
                    ),
                    account_type: from_value(record.account_type).unwrap(),
                    guest_users: None,
                    created: record.created.into(),
                    updated: match record.updated {
                        None => None,
                        Some(res) => Some(DateTime::from(res)),
                    },
                    meta: None,
                }))
            }
            Err(err) => {
                if err.is_prisma_error::<RecordNotFound>() {
                    return updating_err(format!(
                        "Invalid primary key: {:?}",
                        account_id
                    ))
                    .with_exp_true()
                    .as_error();
                };

                return updating_err(format!(
                    "Unexpected error detected on update record: {}",
                    err
                ))
                .as_error();
            }
        }
    }

    async fn update_own_account_name(
        &self,
        account_id: Uuid,
        name: String,
    ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return updating_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Update account
        // ? -------------------------------------------------------------------

        match client
            .account()
            .update(
                account_model::id::equals(account_id.to_string()),
                vec![account_model::name::set(name)],
            )
            .include(account_model::include!({
                owners
                tags: select {
                    id
                    value
                    meta
                }
            }))
            .exec()
            .await
        {
            Ok(record) => {
                let id = Uuid::parse_str(&record.id).unwrap();

                Ok(UpdatingResponseKind::Updated(Account {
                    id: Some(id.to_owned()),
                    name: record.name,
                    slug: record.slug,
                    tags: match record.tags.len() {
                        0 => None,
                        _ => Some(
                            record
                                .tags
                                .to_owned()
                                .into_iter()
                                .map(|i| Tag {
                                    id: Uuid::parse_str(&i.id).unwrap(),
                                    value: i.value,
                                    meta: match i.meta {
                                        None => None,
                                        Some(meta) => {
                                            Some(from_value(meta).unwrap())
                                        }
                                    },
                                })
                                .collect::<Vec<Tag>>(),
                        ),
                    },
                    is_active: record.is_active,
                    is_checked: record.is_checked,
                    is_archived: record.is_archived,
                    verbose_status: Some(VerboseStatus::from_flags(
                        record.is_active,
                        record.is_checked,
                        record.is_archived,
                    )),
                    is_default: record.is_default,
                    owners: Children::Records(
                        record
                            .owners
                            .into_iter()
                            .map(|owner| {
                                User::new(
                                    Some(Uuid::parse_str(&owner.id).unwrap()),
                                    owner.username,
                                    Email::from_string(owner.email).unwrap(),
                                    Some(owner.first_name),
                                    Some(owner.last_name),
                                    owner.is_active,
                                    owner.created.into(),
                                    match owner.updated {
                                        None => None,
                                        Some(date) => {
                                            Some(date.with_timezone(&Local))
                                        }
                                    },
                                    Some(Parent::Id(id)),
                                    None,
                                )
                                .with_principal(owner.is_principal)
                            })
                            .collect::<Vec<User>>(),
                    ),
                    account_type: from_value(record.account_type).unwrap(),
                    guest_users: None,
                    created: record.created.into(),
                    updated: match record.updated {
                        None => None,
                        Some(res) => Some(DateTime::from(res)),
                    },
                    meta: None,
                }))
            }
            Err(err) => {
                if err.is_prisma_error::<RecordNotFound>() {
                    return updating_err(format!(
                        "Invalid primary key: {:?}",
                        account_id
                    ))
                    .with_exp_true()
                    .as_error();
                };

                return updating_err(format!(
                    "Unexpected error detected on update record: {}",
                    err
                ))
                .as_error();
            }
        }
    }

    async fn update_account_type(
        &self,
        account_id: Uuid,
        account_type: AccountType,
    ) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return updating_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Update account
        // ? -------------------------------------------------------------------

        match client
            .account()
            .update(
                account_model::id::equals(account_id.to_string()),
                vec![account_model::account_type::set(
                    to_value(account_type).unwrap(),
                )],
            )
            .include(account_model::include!({
                owners
                tags: select {
                    id
                    value
                    meta
                }
            }))
            .exec()
            .await
        {
            Ok(record) => {
                let id = Uuid::parse_str(&record.id).unwrap();

                Ok(UpdatingResponseKind::Updated(Account {
                    id: Some(id.to_owned()),
                    name: record.name,
                    slug: record.slug,
                    tags: match record.tags.len() {
                        0 => None,
                        _ => Some(
                            record
                                .tags
                                .to_owned()
                                .into_iter()
                                .map(|i| Tag {
                                    id: Uuid::parse_str(&i.id).unwrap(),
                                    value: i.value,
                                    meta: match i.meta {
                                        None => None,
                                        Some(meta) => {
                                            Some(from_value(meta).unwrap())
                                        }
                                    },
                                })
                                .collect::<Vec<Tag>>(),
                        ),
                    },
                    is_active: record.is_active,
                    is_checked: record.is_checked,
                    is_archived: record.is_archived,
                    verbose_status: Some(VerboseStatus::from_flags(
                        record.is_active,
                        record.is_checked,
                        record.is_archived,
                    )),
                    is_default: record.is_default,
                    owners: Children::Records(
                        record
                            .owners
                            .into_iter()
                            .map(|owner| {
                                User::new(
                                    Some(Uuid::parse_str(&owner.id).unwrap()),
                                    owner.username,
                                    Email::from_string(owner.email).unwrap(),
                                    Some(owner.first_name),
                                    Some(owner.last_name),
                                    owner.is_active,
                                    owner.created.into(),
                                    match owner.updated {
                                        None => None,
                                        Some(date) => {
                                            Some(date.with_timezone(&Local))
                                        }
                                    },
                                    Some(Parent::Id(id)),
                                    None,
                                )
                                .with_principal(owner.is_principal)
                            })
                            .collect::<Vec<User>>(),
                    ),
                    account_type: from_value(record.account_type).unwrap(),
                    guest_users: None,
                    created: record.created.into(),
                    updated: match record.updated {
                        None => None,
                        Some(res) => Some(DateTime::from(res)),
                    },
                    meta: None,
                }))
            }
            Err(err) => {
                if err.is_prisma_error::<RecordNotFound>() {
                    return updating_err(format!(
                        "Invalid primary key: {:?}",
                        account_id
                    ))
                    .with_exp_true()
                    .as_error();
                };

                return updating_err(format!(
                    "Unexpected error detected on update record: {}",
                    err
                ))
                .as_error();
            }
        }
    }

    async fn update_account_meta(
        &self,
        account_id: Uuid,
        key: AccountMetaKey,
        value: String,
    ) -> Result<UpdatingResponseKind<AccountMeta>, MappedErrors> {
        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return updating_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        match client
            ._transaction()
            .run(|client| async move {
                let tenant = client
                    .account()
                    .find_unique(account_model::id::equals(
                        account_id.to_string(),
                    ))
                    .select(account_model::select!({ meta }))
                    .exec()
                    .await?;

                let empty_map = AccountMeta::new();
                let mut updated_meta: AccountMeta = if let Some(data) = tenant {
                    match data.meta.to_owned() {
                        Some(meta) => from_value(meta).unwrap(),
                        None => empty_map,
                    }
                } else {
                    empty_map
                };

                updated_meta.insert(key, value);

                client
                    .account()
                    .update(
                        account_model::id::equals(account_id.to_string()),
                        vec![account_model::meta::set(Some(
                            to_value(updated_meta.to_owned()).unwrap(),
                        ))],
                    )
                    .exec()
                    .await?;

                Ok::<HashMap<AccountMetaKey, String>, QueryError>(updated_meta)
            })
            .await
        {
            Ok(record) => Ok(UpdatingResponseKind::Updated(record)),
            Err(err) => updating_err(format!("Could not create tenant: {err}"))
                .as_error(),
        }
    }
}
