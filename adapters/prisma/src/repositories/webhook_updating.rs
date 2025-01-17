use crate::{
    prisma::webhook as webhook_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::Local;
use myc_core::domain::{
    dtos::{native_error_codes::NativeErrorCodes, webhook::WebHook},
    entities::WebHookUpdating,
};
use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{updating_err, MappedErrors},
};
use prisma_client_rust::prisma_errors::query_engine::RecordNotFound;
use serde_json::from_value;
use shaku::Component;
use std::{process::id as process_id, str::FromStr};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = WebHookUpdating)]
pub struct WebHookUpdatingSqlDbRepository {}

#[async_trait]
impl WebHookUpdating for WebHookUpdatingSqlDbRepository {
    async fn update(
        &self,
        webhook: WebHook,
    ) -> Result<UpdatingResponseKind<WebHook>, MappedErrors> {
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

        let webhook_id = match webhook.id {
            None => {
                return updating_err(String::from(
                    "Unable to update webhook. Invalid record ID",
                ))
                .as_error()
            }
            Some(res) => res,
        };

        match client
            .webhook()
            .update(
                webhook_model::id::equals(webhook_id.to_string()),
                vec![
                    webhook_model::name::set(webhook.name),
                    webhook_model::description::set(
                        webhook.description.to_owned(),
                    ),
                    webhook_model::url::set(webhook.url.to_owned()),
                    webhook_model::trigger::set(
                        webhook.trigger.to_owned().to_string(),
                    ),
                    webhook_model::is_active::set(webhook.is_active),
                ],
            )
            .exec()
            .await
        {
            Ok(record) => {
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

                Ok(UpdatingResponseKind::Updated(webhook))
            }
            Err(err) => {
                if err.is_prisma_error::<RecordNotFound>() {
                    return updating_err(format!(
                        "Invalid primary key: {:?}",
                        webhook_id
                    ))
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
}
