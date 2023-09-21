use crate::{prisma::user as user_model, repositories::connector::get_client};

use async_trait::async_trait;
use chrono::Local;
use clean_base::{
    dtos::Parent,
    entities::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::{factories::creation_err, MappedErrors},
};
use myc_core::domain::{
    dtos::{email::Email, native_error_codes::NativeErrorCodes, user::User},
    entities::UserRegistration,
};
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = UserRegistration)]
pub struct UserRegistrationSqlDbRepository {}

#[async_trait]
impl UserRegistration for UserRegistrationSqlDbRepository {
    async fn get_or_create(
        &self,
        user: User,
    ) -> Result<GetOrCreateResponseKind<User>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return creation_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001.as_str())
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Build the initial query (get part of the get-or-create)
        // ? -------------------------------------------------------------------

        let response = client
            .user()
            .find_first(vec![user_model::email::equals(user.email.get_email())])
            .exec()
            .await;

        match response.unwrap() {
            Some(record) => {
                let record = record;
                let id = Uuid::parse_str(&record.id);

                return Ok(GetOrCreateResponseKind::NotCreated(
                    User::new(
                        Some(id.unwrap()),
                        record.username,
                        Email::from_string(record.email)?,
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
                    )
                    .with_principal(record.is_principal),
                    "User already exists".to_string(),
                ));
            }
            None => (),
        };

        // ? -------------------------------------------------------------------
        // ? Build create part of the get-or-create
        // ? -------------------------------------------------------------------

        let account_id = match user.to_owned().account {
            None => {
                return creation_err(String::from(
                    "Account ID is required to create a user",
                ))
                .with_code(NativeErrorCodes::MYC00002.as_str())
                .as_error()
            }
            Some(record) => match record {
                Parent::Id(id) => id,
                Parent::Record(record) => match record.id {
                    None => {
                        return creation_err(String::from(
                            "Unable to create user. Invalid account ID",
                        ))
                        .with_exp_true()
                        .as_error()
                    }
                    Some(id) => id,
                },
            },
        };

        let response = client
            .user()
            .create(
                user.to_owned().username,
                user.to_owned().email.get_email(),
                user.to_owned().first_name.unwrap_or(String::from("")),
                user.to_owned().last_name.unwrap_or(String::from("")),
                vec![
                    user_model::is_active::set(user.is_active),
                    user_model::is_principal::set(user.is_principal()),
                ],
            )
            .exec()
            .await;

        match response {
            Ok(record) => {
                let record = record;
                let id = Uuid::parse_str(&record.id);

                Ok(GetOrCreateResponseKind::Created(
                    User::new(
                        Some(id.unwrap()),
                        record.username,
                        Email::from_string(record.email)?,
                        Some(record.first_name),
                        Some(record.last_name),
                        record.is_active,
                        record.created.into(),
                        match record.updated {
                            None => None,
                            Some(date) => Some(date.with_timezone(&Local)),
                        },
                        Some(Parent::Id(account_id)),
                    )
                    .with_principal(record.is_principal),
                ))
            }
            Err(err) => {
                return creation_err(format!(
                    "Unexpected error detected on update record: {}",
                    err
                ))
                .as_error();
            }
        }
    }

    // ? -----------------------------------------------------------------------
    // ! NOT IMPLEMENTED METHODS
    // ? -----------------------------------------------------------------------

    async fn create(
        &self,
        user: User,
    ) -> Result<CreateResponseKind<User>, MappedErrors> {
        self.create(user).await
    }
}
