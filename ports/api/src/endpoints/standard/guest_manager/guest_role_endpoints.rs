use crate::{
    dtos::MyceliumProfileData,
    endpoints::{shared::UrlGroup, standard::shared::build_actor_context},
    modules::{
        GuestRoleDeletionModule, GuestRoleFetchingModule,
        GuestRoleRegistrationModule, GuestRoleUpdatingModule,
    },
};

use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        actors::ActorName,
        dtos::guest_role::Permissions,
        entities::{
            GuestRoleDeletion, GuestRoleFetching, GuestRoleRegistration,
            GuestRoleUpdating,
        },
    },
    use_cases::roles::standard::guest_manager::guest_role::{
        create_guest_role, delete_guest_role, list_guest_roles,
        update_guest_role_name_and_description, update_guest_role_permissions,
        ActionType,
    },
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        delete_response_kind, fetch_many_response_kind,
        get_or_create_response_kind, updating_response_kind,
    },
};
use serde::Deserialize;
use shaku_actix::Inject;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ? -----------------------------------------------------------------------
// ? Configure application
// ? -----------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(crate_guest_role_url)
        .service(list_guest_roles_url)
        .service(delete_guest_role_url)
        .service(update_guest_role_name_and_description_url)
        .service(update_guest_role_permissions_url);
}

// ? -----------------------------------------------------------------------
// ? Define API structs
// ? -----------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateGuestRoleBody {
    pub name: String,
    pub description: String,
    pub role_id: Uuid,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListGuestRolesParams {
    pub name: Option<String>,
    pub role_id: Option<Uuid>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGuestRoleNameAndDescriptionBody {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGuestRolePermissionsBody {
    pub permission: Permissions,
    pub action_type: ActionType,
}

/// Create Guest Role
///
/// Guest Roles provide permissions to simple Roles.
#[utoipa::path(
    post,
    context_path = build_actor_context(ActorName::GuestManager, UrlGroup::GuestRoles),
    request_body = CreateGuestRoleBody,
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = HttpJsonResponse,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = HttpJsonResponse,
        ),
        (
            status = 201,
            description = "Guest Role created.",
            body = GuestRole,
        ),
        (
            status = 200,
            description = "Guest Role already exists.",
            body = GuestRole,
        ),
    ),
)]
#[post("/")]
pub async fn crate_guest_role_url(
    json: web::Json<CreateGuestRoleBody>,
    profile: MyceliumProfileData,
    role_registration_repo: Inject<
        GuestRoleRegistrationModule,
        dyn GuestRoleRegistration,
    >,
) -> impl Responder {
    match create_guest_role(
        profile.to_profile(),
        json.name.to_owned(),
        json.description.to_owned(),
        json.role_id.to_owned(),
        None,
        Box::new(&*role_registration_repo),
    )
    .await
    {
        Ok(res) => get_or_create_response_kind(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}

/// List Roles
#[utoipa::path(
    get,
    context_path = build_actor_context(ActorName::GuestManager, UrlGroup::GuestRoles),
    params(
        ListGuestRolesParams,
    ),
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "Not found.",
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = HttpJsonResponse,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = HttpJsonResponse,
        ),
        (
            status = 200,
            description = "Success.",
            body = [Role],
        ),
    ),
)]
#[get("/")]
pub async fn list_guest_roles_url(
    info: web::Query<ListGuestRolesParams>,
    profile: MyceliumProfileData,
    guest_role_fetching_repo: Inject<
        GuestRoleFetchingModule,
        dyn GuestRoleFetching,
    >,
) -> impl Responder {
    match list_guest_roles(
        profile.to_profile(),
        info.name.to_owned(),
        info.role_id.to_owned(),
        Box::new(&*guest_role_fetching_repo),
    )
    .await
    {
        Ok(res) => fetch_many_response_kind(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}

/// Delete Guest Role
///
/// Delete a single guest role.
#[utoipa::path(
    delete,
    context_path = build_actor_context(ActorName::GuestManager, UrlGroup::GuestRoles),
    params(
        ("role" = Uuid, Path, description = "The guest-role primary key."),
    ),
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = HttpJsonResponse,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = HttpJsonResponse,
        ),
        (
            status = 400,
            description = "Guest Role not deleted.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "Guest Role deleted.",
        ),
    ),
)]
#[delete("/{role}")]
pub async fn delete_guest_role_url(
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    role_deletion_repo: Inject<GuestRoleDeletionModule, dyn GuestRoleDeletion>,
) -> impl Responder {
    match delete_guest_role(
        profile.to_profile(),
        path.to_owned(),
        Box::new(&*role_deletion_repo),
    )
    .await
    {
        Ok(res) => delete_response_kind(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}

/// Partial Update Guest Role
///
/// Update name and description of a single Guest Role.
#[utoipa::path(
    patch,
    context_path = build_actor_context(ActorName::GuestManager, UrlGroup::GuestRoles),
    params(
        ("role" = Uuid, Path, description = "The guest-role primary key."),
    ),
    request_body = UpdateGuestRoleNameAndDescriptionBody,
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = HttpJsonResponse,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = HttpJsonResponse,
        ),
        (
            status = 400,
            description = "Guest Role not deleted.",
            body = HttpJsonResponse,
        ),
        (
            status = 202,
            description = "Guest Role updated.",
            body = GuestRole,
        ),
    ),
)]
#[patch("/{role}")]
pub async fn update_guest_role_name_and_description_url(
    path: web::Path<Uuid>,
    body: web::Json<UpdateGuestRoleNameAndDescriptionBody>,
    profile: MyceliumProfileData,
    role_fetching_repo: Inject<GuestRoleFetchingModule, dyn GuestRoleFetching>,
    role_updating_repo: Inject<GuestRoleUpdatingModule, dyn GuestRoleUpdating>,
) -> impl Responder {
    match update_guest_role_name_and_description(
        profile.to_profile(),
        body.name.to_owned(),
        body.description.to_owned(),
        path.to_owned(),
        Box::new(&*role_fetching_repo),
        Box::new(&*role_updating_repo),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}

/// Change permissions of Guest Role
///
/// Upgrade or Downgrade permissions of Guest Role.
#[utoipa::path(
    patch,
    context_path = build_actor_context(ActorName::GuestManager, UrlGroup::GuestRoles),
    params(
        ("role" = Uuid, Path, description = "The guest-role primary key."),
    ),
    request_body = UpdateGuestRolePermissionsBody,
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = HttpJsonResponse,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = HttpJsonResponse,
        ),
        (
            status = 400,
            description = "Guest Role not deleted.",
            body = HttpJsonResponse,
        ),
        (
            status = 202,
            description = "Guest Role updated.",
            body = GuestRole,
        ),
    ),
)]
#[patch("/{role}/permissions")]
pub async fn update_guest_role_permissions_url(
    path: web::Path<Uuid>,
    body: web::Json<UpdateGuestRolePermissionsBody>,
    profile: MyceliumProfileData,
    role_fetching_repo: Inject<GuestRoleFetchingModule, dyn GuestRoleFetching>,
    role_updating_repo: Inject<GuestRoleUpdatingModule, dyn GuestRoleUpdating>,
) -> impl Responder {
    match update_guest_role_permissions(
        profile.to_profile(),
        path.to_owned(),
        body.permission.to_owned(),
        body.action_type.to_owned(),
        Box::new(&*role_fetching_repo),
        Box::new(&*role_updating_repo),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}
