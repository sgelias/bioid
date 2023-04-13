// Export from this module
mod azure;
pub use azure::check_credentials::check_credentials as az_check_credentials;

mod google;
pub use google::check_credentials::check_credentials as gc_check_credentials;
