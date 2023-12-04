use crate::domain::{dtos::profile::Profile, entities::RoleDeletion};

use clean_base::{
    entities::DeletionResponseKind,
    utils::errors::{factories::use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Delete a single role.
pub async fn delete_role(
    profile: Profile,
    role_id: Uuid,
    role_deletion_repo: Box<&dyn RoleDeletion>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? ----------------------------------------------------------------------

    if !profile.is_manager {
        return use_case_err(
            "The current user has no sufficient privileges to delete roles."
                .to_string(),
        )
        .as_error();
    }

    // ? ----------------------------------------------------------------------
    // ? Persist Role
    // ? ----------------------------------------------------------------------

    role_deletion_repo.delete(role_id).await
}