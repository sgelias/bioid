use crate::{
    dtos::MyceliumProfileData,
    endpoints::{shared::UrlGroup, standard::shared::build_actor_context},
    modules::{
        TenantDeletionModule, TenantFetchingModule, TenantUpdatingModule,
        UserFetchingModule,
    },
};

use actix_web::{delete, post, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        actors::ActorName,
        entities::{
            TenantDeletion, TenantFetching, TenantUpdating, UserFetching,
        },
    },
    use_cases::roles::standard::tenant_owner::{
        guest_tenant_owner, revoke_tenant_owner,
    },
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        create_response_kind, delete_response_kind,
    },
    Email,
};
use serde::Deserialize;
use shaku_actix::Inject;
use utoipa::ToSchema;
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(guest_tenant_owner_url)
        .service(revoke_tenant_owner_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GuestTenantOwnerBody {
    email: String,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    context_path = build_actor_context(ActorName::TenantOwner, UrlGroup::Owners),
    params(
        ("tenant_id" = Uuid, Path, description = "The tenant primary key."),
    ),
    request_body = GuestTenantOwnerBody,
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
            description = "Owner already exists.",
            body = HttpJsonResponse,
        ),
        (
            status = 201,
            description = "Owner created.",
            body = TenantOwnerConnection,
        ),
    ),
)]
#[post("/{tenant_id}")]
pub async fn guest_tenant_owner_url(
    path: web::Path<Uuid>,
    body: web::Json<GuestTenantOwnerBody>,
    profile: MyceliumProfileData,
    owner_fetching_repo: Inject<UserFetchingModule, dyn UserFetching>,
    tenant_updating_repo: Inject<TenantUpdatingModule, dyn TenantUpdating>,
) -> impl Responder {
    let email = match Email::from_string(body.email.to_owned()) {
        Ok(email) => email,
        Err(err) => {
            return HttpResponse::BadRequest()
                .json(HttpJsonResponse::new_message(err.to_string()))
        }
    };

    match guest_tenant_owner(
        profile.to_profile(),
        email,
        path.into_inner(),
        Box::new(&*owner_fetching_repo),
        Box::new(&*tenant_updating_repo),
    )
    .await
    {
        Ok(res) => create_response_kind(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}

#[utoipa::path(
    delete,
    context_path = build_actor_context(ActorName::TenantOwner, UrlGroup::Owners),
    params(
        ("tenant_id" = Uuid, Path, description = "The tenant primary key."),
    ),
    request_body = GuestTenantOwnerBody,
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
            description = "Owner deleted.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "Owner deleted.",
        ),
    ),
)]
#[delete("/{tenant_id}")]
pub async fn revoke_tenant_owner_url(
    path: web::Path<Uuid>,
    body: web::Json<GuestTenantOwnerBody>,
    profile: MyceliumProfileData,
    tenant_fetching_repo: Inject<TenantFetchingModule, dyn TenantFetching>,
    tenant_deletion_repo: Inject<TenantDeletionModule, dyn TenantDeletion>,
) -> impl Responder {
    let email = match Email::from_string(body.email.to_owned()) {
        Ok(email) => email,
        Err(err) => {
            return HttpResponse::BadRequest()
                .json(HttpJsonResponse::new_message(err.to_string()))
        }
    };

    match revoke_tenant_owner(
        profile.to_profile(),
        email,
        path.into_inner(),
        Box::new(&*tenant_fetching_repo),
        Box::new(&*tenant_deletion_repo),
    )
    .await
    {
        Ok(res) => delete_response_kind(res),
        Err(err) => HttpResponse::InternalServerError()
            .json(HttpJsonResponse::new_message(err.to_string())),
    }
}
