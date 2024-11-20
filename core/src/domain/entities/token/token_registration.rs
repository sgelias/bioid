use crate::domain::dtos::token::{
    AccountScopedConnectionString, EmailConfirmationTokenMeta,
    PasswordChangeTokenMeta, RoleScopedConnectionString, Token,
};

use async_trait::async_trait;
use chrono::{DateTime, Local};
use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait TokenRegistration: Interface + Send + Sync {
    async fn create_email_confirmation_token(
        &self,
        meta: EmailConfirmationTokenMeta,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<Token>, MappedErrors>;

    async fn create_password_change_token(
        &self,
        meta: PasswordChangeTokenMeta,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<Token>, MappedErrors>;

    async fn create_account_scoped_connection_string(
        &self,
        meta: AccountScopedConnectionString,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<Token>, MappedErrors>;

    async fn create_role_scoped_connection_string(
        &self,
        meta: RoleScopedConnectionString,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<Token>, MappedErrors>;
}
