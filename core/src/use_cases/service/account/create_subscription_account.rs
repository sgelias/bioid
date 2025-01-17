use crate::{
    domain::{
        actors::SystemActor::*,
        dtos::{
            account::Account,
            guest_role::Permission,
            native_error_codes::NativeErrorCodes,
            token::TenantScopedConnectionString,
            webhook::{WebHookPropagationResponse, WebHookTrigger},
        },
        entities::{AccountRegistration, WebHookFetching},
    },
    models::AccountLifeCycle,
    use_cases::support::dispatch_webhooks,
};

use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Create an account flagged as subscription.
///
/// Subscription accounts represents results centering accounts.
#[tracing::instrument(
    name = "create_subscription_account",
    fields(user_id = %scope.user_id),
    skip(scope, account_registration_repo, webhook_fetching_repo)
)]
pub async fn create_subscription_account(
    scope: TenantScopedConnectionString,
    tenant_id: Uuid,
    account_name: String,
    config: AccountLifeCycle,
    account_registration_repo: Box<&dyn AccountRegistration>,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
) -> Result<WebHookPropagationResponse<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    scope.contain_enough_permissions(
        tenant_id,
        vec![
            (TenantManager.to_string(), Permission::Write),
            (SubscriptionsManager.to_string(), Permission::Write),
        ],
    )?;

    // ? -----------------------------------------------------------------------
    // ? Register the account
    //
    // The account are registered using the already created user.
    // ? -----------------------------------------------------------------------

    let mut unchecked_account =
        Account::new_subscription_account(account_name, tenant_id);

    unchecked_account.is_checked = true;

    let account = match account_registration_repo
        .create_subscription_account(unchecked_account, tenant_id)
        .await?
    {
        CreateResponseKind::NotCreated(account, msg) => {
            return use_case_err(format!("({}): {}", account.name, msg))
                .with_code(NativeErrorCodes::MYC00003)
                .as_error()
        }
        CreateResponseKind::Created(account) => account,
    };

    // ? -----------------------------------------------------------------------
    // ? Propagate account
    // ? -----------------------------------------------------------------------

    let responses = dispatch_webhooks(
        WebHookTrigger::CreateSubscriptionAccount,
        account.to_owned(),
        config,
        webhook_fetching_repo,
    )
    .await;

    Ok(responses)
}
