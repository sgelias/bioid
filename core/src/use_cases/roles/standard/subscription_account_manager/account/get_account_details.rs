use crate::domain::{
    actors::DefaultActors,
    dtos::{account::Account, profile::Profile},
    entities::AccountFetching,
};

use clean_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use uuid::Uuid;

/// Get details of a single account
///
/// These details could include information about guest accounts, modifications
/// and others.
pub async fn get_account_details(
    profile: Profile,
    account_id: Uuid,
    account_fetching_repo: Box<&dyn AccountFetching>,
) -> Result<FetchResponseKind<Account, Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile.get_create_ids_or_error(vec![
        DefaultActors::SubscriptionAccountManager.to_string(),
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Fetch account
    // ? -----------------------------------------------------------------------

    account_fetching_repo.get(account_id).await
}