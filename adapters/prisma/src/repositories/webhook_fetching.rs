use crate::{
    prisma::webhook as webhook_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::Local;
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        webhook::{WebHook, WebHookTrigger},
    },
    entities::WebHookFetching,
};
use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
use prisma_client_rust::{and, operator::and as and_o};
use serde_json::from_value;
use shaku::Component;
use std::{process::id as process_id, str::FromStr};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = WebHookFetching)]
pub struct WebHookFetchingSqlDbRepository {}

#[async_trait]
impl WebHookFetching for WebHookFetchingSqlDbRepository {
    async fn get(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<WebHook, Uuid>, MappedErrors> {
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
            .webhook()
            .find_unique(webhook_model::id::equals(id.to_owned().to_string()))
            .exec()
            .await
        {
            Err(err) => {
                return fetching_err(format!(
                    "Unexpected error on parse user email: {err}"
                ))
                .as_error()
            }
            Ok(res) => match res {
                None => Ok(FetchResponseKind::NotFound(Some(id))),
                Some(record) => {
                    let mut webhook = WebHook::new(
                        record.name,
                        record.description.into(),
                        record.url,
                        record.trigger.parse().unwrap(),
                        record.secret.map(|secret| from_value(secret).unwrap()),
                    );

                    webhook.id = Some(Uuid::from_str(&record.id).unwrap());
                    webhook.is_active = record.is_active;
                    webhook.created = record.created.into();
                    webhook.updated = match record.updated {
                        None => None,
                        Some(date) => Some(date.with_timezone(&Local)),
                    };

                    webhook.redact_secret_token();

                    Ok(FetchResponseKind::Found(webhook))
                }
            },
        }
    }

    async fn list(
        &self,
        name: Option<String>,
        trigger: Option<WebHookTrigger>,
    ) -> Result<FetchManyResponseKind<WebHook>, MappedErrors> {
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
        // ? Build list query statement
        // ? -------------------------------------------------------------------

        let mut and_stmt = vec![];
        let mut query_stmt = vec![];

        if name.is_some() {
            and_stmt.push(webhook_model::name::contains(name.unwrap()))
        }

        if trigger.is_some() {
            and_stmt.push(webhook_model::trigger::contains(
                trigger.unwrap().to_string(),
            ))
        }

        if !and_stmt.is_empty() {
            query_stmt.push(and_o(and_stmt))
        }

        // ? -------------------------------------------------------------------
        // ? Get the user
        // ? -------------------------------------------------------------------

        match client.webhook().find_many(query_stmt).exec().await {
            Err(err) => {
                return fetching_err(format!(
                    "Unexpected error on fetch webhooks: {err}",
                ))
                .as_error()
            }
            Ok(res) => {
                let response = res
                    .into_iter()
                    .map(|record| {
                        let mut webhook = WebHook::new(
                            record.name,
                            record.description.into(),
                            record.url,
                            record.trigger.parse().unwrap(),
                            record
                                .secret
                                .map(|secret| from_value(secret).unwrap()),
                        );

                        webhook.id = Some(Uuid::from_str(&record.id).unwrap());
                        webhook.is_active = record.is_active;
                        webhook.created = record.created.into();
                        webhook.updated = match record.updated {
                            None => None,
                            Some(date) => Some(date.with_timezone(&Local)),
                        };

                        webhook.redact_secret_token();

                        webhook
                    })
                    .collect::<Vec<WebHook>>();

                if response.len() == 0 {
                    return Ok(FetchManyResponseKind::NotFound);
                }

                Ok(FetchManyResponseKind::Found(response))
            }
        }
    }

    async fn list_by_trigger(
        &self,
        trigger: WebHookTrigger,
    ) -> Result<FetchManyResponseKind<WebHook>, MappedErrors> {
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
            .webhook()
            .find_many(vec![and![
                webhook_model::trigger::equals(trigger.to_string()),
                webhook_model::is_active::equals(true),
            ]])
            .exec()
            .await
        {
            Err(err) => {
                return fetching_err(format!(
                    "Unexpected error on fetch webhooks: {err}",
                ))
                .as_error()
            }
            Ok(res) => {
                let response = res
                    .into_iter()
                    .map(|record| {
                        let mut webhook = WebHook::new(
                            record.name,
                            record.description.into(),
                            record.url,
                            record.trigger.parse().unwrap(),
                            record
                                .secret
                                .map(|secret| from_value(secret).unwrap()),
                        );

                        webhook.id = Some(Uuid::from_str(&record.id).unwrap());
                        webhook.is_active = record.is_active;
                        webhook.created = record.created.into();
                        webhook.updated = match record.updated {
                            None => None,
                            Some(date) => Some(date.with_timezone(&Local)),
                        };

                        webhook
                    })
                    .collect::<Vec<WebHook>>();

                if response.len() == 0 {
                    return Ok(FetchManyResponseKind::NotFound);
                }

                Ok(FetchManyResponseKind::Found(response))
            }
        }
    }
}
