use crate::{dtos::MyceliumProfileData, modules::RoutesFetchingModule};

use actix_web::{get, web, Responder};
use myc_core::{
    domain::{dtos::service::Service, entities::RoutesFetching},
    use_cases::role_scoped::gateway_manager::service::list_services,
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        fetch_many_response_kind, handle_mapped_error,
    },
};
use serde::Deserialize;
use shaku_actix::Inject;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(list_services_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListServicesParams {
    id: Option<Uuid>,
    name: Option<String>,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// List routes by service
///
/// This function is restricted to the GatewayManager users. List routes by
/// service name or service id.
///
#[utoipa::path(
    get,
    params(
        ListServicesParams,
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
            status = 204,
            description = "Not found.",
        ),
        (
            status = 200,
            description = "Fetching success.",
            body = [Service],
        ),
    ),
)]
#[get("")]
pub async fn list_services_url(
    query: web::Query<ListServicesParams>,
    profile: MyceliumProfileData,
    routes_fetching_repo: Inject<RoutesFetchingModule, dyn RoutesFetching>,
) -> impl Responder {
    match list_services(
        profile.to_profile(),
        query.id.to_owned(),
        query.name.to_owned(),
        Box::new(&*routes_fetching_repo),
    )
    .await
    {
        Ok(res) => fetch_many_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}