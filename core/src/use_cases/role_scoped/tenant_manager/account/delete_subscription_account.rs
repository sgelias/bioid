use crate::domain::{
    actors::SystemActor, dtos::profile::Profile, entities::AccountDeletion,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "delete_subscription_account",
    fields(profile_id = %profile.acc_id),
    skip(account_deletion_repo)
)]
pub async fn delete_subscription_account(
    profile: Profile,
    tenant_id: Uuid,
    account_id: Uuid,
    account_deletion_repo: Box<&dyn AccountDeletion>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .on_tenant(tenant_id)
        .get_related_account_with_default_write_or_error(vec![
            SystemActor::TenantManager.to_string(),
        ])?;

    // ? -----------------------------------------------------------------------
    // ? Delete account
    // ? -----------------------------------------------------------------------

    account_deletion_repo
        .delete(account_id, related_accounts)
        .await
}