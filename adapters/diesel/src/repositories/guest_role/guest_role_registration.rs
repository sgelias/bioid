use super::shared::map_model_to_dto;
use crate::{
    models::{config::DbConfig, guest_role::GuestRole as GuestRoleModel},
    schema::guest_role as guest_role_model,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{guest_role::GuestRole, native_error_codes::NativeErrorCodes},
    entities::GuestRoleRegistration,
};
use mycelium_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = GuestRoleRegistration)]
pub struct GuestRoleRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbConfig>,
}

#[async_trait]
impl GuestRoleRegistration for GuestRoleRegistrationSqlDbRepository {
    async fn get_or_create(
        &self,
        guest_role: GuestRole,
    ) -> Result<GetOrCreateResponseKind<GuestRole>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // Check if role already exists
        let existing = guest_role_model::table
            .filter(guest_role_model::name.eq(&guest_role.name))
            .select(GuestRoleModel::as_select())
            .first::<GuestRoleModel>(conn)
            .optional()
            .map_err(|e| {
                creation_err(format!("Failed to check existing role: {}", e))
            })?;

        if let Some(record) = existing {
            return Ok(GetOrCreateResponseKind::NotCreated(
                map_model_to_dto(record),
                "Role already exists".to_string(),
            ));
        }

        // Create new role
        let new_role = GuestRoleModel {
            id: uuid::Uuid::new_v4(),
            name: guest_role.name,
            slug: guest_role.slug,
            description: guest_role.description,
            permission: guest_role.permission.to_i32(),
        };

        let created = diesel::insert_into(guest_role_model::table)
            .values(&new_role)
            .get_result::<GuestRoleModel>(conn)
            .map_err(|e| {
                creation_err(format!("Failed to create role: {}", e))
            })?;

        Ok(GetOrCreateResponseKind::Created(map_model_to_dto(created)))
    }
}