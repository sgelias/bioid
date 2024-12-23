use crate::domain::{
    actors::SystemActor,
    dtos::{guest_role::GuestRole, profile::Profile},
    entities::GuestRoleUpdating,
};

use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(name = "remove_role_child", skip_all)]
pub async fn remove_role_child(
    profile: Profile,
    role_id: Uuid,
    child_id: Uuid,
    guest_role_updating_repo: Box<&dyn GuestRoleUpdating>,
) -> Result<UpdatingResponseKind<Option<GuestRole>>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? ----------------------------------------------------------------------

    profile.get_default_write_ids_or_error(vec![SystemActor::GuestManager])?;

    // ? ----------------------------------------------------------------------
    // ? Persist UserRole
    // ? ----------------------------------------------------------------------

    guest_role_updating_repo
        .remove_role_child(role_id, child_id)
        .await
}
