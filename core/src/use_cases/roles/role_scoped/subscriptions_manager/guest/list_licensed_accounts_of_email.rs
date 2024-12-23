use crate::domain::{
    actors::SystemActor,
    dtos::{
        email::Email,
        profile::{LicensedResource, Profile},
        route_type::PermissionedRoles,
    },
    entities::LicensedResourcesFetching,
};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

/// Get all licenses related to email
///
/// Fetch all subscription accounts which an email was guest.
#[tracing::instrument(
    name = "list_licensed_accounts_of_email",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn list_licensed_accounts_of_email(
    profile: Profile,
    tenant_id: Uuid,
    email: Email,
    roles: Option<Vec<String>>,
    was_verified: Option<bool>,
    permissioned_roles: Option<PermissionedRoles>,
    licensed_resources_fetching_repo: Box<&dyn LicensedResourcesFetching>,
) -> Result<FetchManyResponseKind<LicensedResource>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .on_tenant(tenant_id)
        .get_related_account_with_default_read_or_error(vec![
            SystemActor::TenantOwner.to_string(),
            SystemActor::TenantManager.to_string(),
            SystemActor::SubscriptionsManager.to_string(),
        ])?;

    // ? -----------------------------------------------------------------------
    // ? Fetch subscriptions from email
    // ? -----------------------------------------------------------------------

    licensed_resources_fetching_repo
        .list(
            email,
            roles,
            permissioned_roles,
            Some(related_accounts),
            was_verified,
        )
        .await
}