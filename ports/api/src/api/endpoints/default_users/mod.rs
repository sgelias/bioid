pub mod account_endpoints;
pub mod profile_endpoints;
pub mod user_endpoints;

use super::shared::SecurityAddon;

use clean_base::dtos::enums::{ChildrenEnum, ParentEnum};
use myc_core::{
    domain::dtos::{
        account::{Account, AccountType, AccountTypeEnum, VerboseStatus},
        email::Email,
        guest::Permissions,
        profile::{LicensedResources, Profile},
        user::User,
    },
    use_cases::roles::default_users::user::EmailRegistrationStatus,
};
use myc_http_tools::utils::JsonError;
use utoipa::OpenApi;

// ? ---------------------------------------------------------------------------
// ? Configure the API documentation
// ? ---------------------------------------------------------------------------

#[derive(OpenApi)]
#[openapi(
    paths(
        account_endpoints::create_default_account_url,
        account_endpoints::update_own_account_name_url,
        profile_endpoints::fetch_profile,
        user_endpoints::check_email_registration_status_url,
    ),
    modifiers(&SecurityAddon),
    components(
        schemas(
            // Default relationship enumerators.
            ChildrenEnum<String, String>,
            ParentEnum<String, String>,

            // Schema models.
            Account,
            AccountType,
            AccountTypeEnum,
            JsonError,
            LicensedResources,
            Profile,
            Permissions,
            VerboseStatus,
            User,
            Email,
            EmailRegistrationStatus,
        ),
    ),
    tags(
        (
            name = "default-users",
            description = "Default Users management endpoints."
        )
    ),
)]
pub struct ApiDoc;
