pub mod middleware;
pub mod providers;
pub mod responses;

pub mod settings;
pub mod utils;

/// This is a re-exportation from the myc core to allow users to import both
/// from myc-api instead of the myc-core.
pub use myc_core::{
    domain::dtos::{
        email::Email,
        guest::PermissionsType,
        profile::{LicensedResources, Profile},
    },
    settings::DEFAULT_PROFILE_KEY,
};
