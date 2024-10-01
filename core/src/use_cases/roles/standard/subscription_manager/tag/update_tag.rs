use crate::domain::{
    actors::DefaultActor,
    dtos::{profile::Profile, tag::Tag},
    entities::TagUpdating,
};

use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};

#[tracing::instrument(name = "update_tag", skip_all)]
pub async fn update_tag(
    profile: Profile,
    tag: Tag,
    tag_updating_repo: Box<&dyn TagUpdating>,
) -> Result<UpdatingResponseKind<Tag>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile.get_default_create_ids_or_error(vec![
        DefaultActor::SubscriptionManager.to_string(),
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Register tag
    // ? -----------------------------------------------------------------------

    tag_updating_repo.update(tag).await
}