use crate::{
    domain::{
        actors::SystemActor::*,
        dtos::{
            account::Account, guest_role::Permission, guest_user::GuestUser,
            native_error_codes::NativeErrorCodes,
            token::RoleScopedConnectionString, user::User,
        },
        entities::{
            AccountRegistration, GuestRoleFetching, GuestUserRegistration,
            MessageSending,
        },
    },
    models::AccountLifeCycle,
    use_cases::support::{
        get_or_create_role_related_account, send_email_notification,
    },
};

use futures::future;
use mycelium_base::{
    dtos::{Children, Parent},
    entities::{FetchResponseKind, GetOrCreateResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use tracing::{info, warn};
use uuid::Uuid;

/// Guest a user to a default account
///
/// This method should be called from webhooks to propagate a new user to a
/// default account.
#[tracing::instrument(name = "guest_to_default_account", skip_all)]
pub async fn guest_to_default_account(
    scope: RoleScopedConnectionString,
    role_id: Uuid,
    account: Account,
    tenant_id: Uuid,
    life_cycle_settings: AccountLifeCycle,
    account_registration_repo: Box<&dyn AccountRegistration>,
    guest_role_fetching_repo: Box<&dyn GuestRoleFetching>,
    message_sending_repo: Box<&dyn MessageSending>,
    guest_user_registration_repo: Box<&dyn GuestUserRegistration>,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check permissions
    // ? -----------------------------------------------------------------------

    scope.contain_enough_permissions(
        tenant_id,
        role_id,
        vec![
            (GuestManager.to_string(), Permission::Write),
            (SubscriptionsManager.to_string(), Permission::Write),
        ],
    )?;

    // ? -----------------------------------------------------------------------
    // ? Guarantee needed information to evaluate guesting
    //
    // Check if the target account is a subscription account or a standard role
    // associated account. Only these accounts can receive guesting. Already
    // check the role_id to be a guest role is valid and exists.
    //
    // ? -----------------------------------------------------------------------

    let (target_account_response, target_role_response) = future::join(
        get_or_create_role_related_account(
            tenant_id,
            role_id,
            account_registration_repo,
        ),
        guest_role_fetching_repo.get(role_id),
    )
    .await;

    let default_subscription_account = match target_account_response? {
        GetOrCreateResponseKind::NotCreated(account, _) => account,
        GetOrCreateResponseKind::Created(account) => account,
    };

    let target_role = match target_role_response? {
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!(
                "Guest role not found: {:?}",
                id.unwrap()
            ))
            .with_code(NativeErrorCodes::MYC00012)
            .as_error()
        }
        FetchResponseKind::Found(role) => role,
    };

    // ? -----------------------------------------------------------------------
    // ? Persist changes
    // ? -----------------------------------------------------------------------

    let guest_email = match account.owners {
        Children::Ids(_) => {
            return use_case_err("Invalid account owner".to_string()).as_error()
        }
        Children::Records(owners) => owners
            .into_iter()
            .filter(|owner| owner.is_principal())
            .collect::<Vec<User>>()
            .first()
            .unwrap()
            .email
            .to_owned(),
    };

    match guest_user_registration_repo
        .get_or_create(
            GuestUser::new_unverified(
                guest_email.to_owned(),
                Parent::Id(role_id),
                None,
            ),
            match default_subscription_account.id {
                None => {
                    warn!(
                        "Default account maybe invalid. ID not found: {:?}",
                        default_subscription_account
                    );

                    return use_case_err("Invalid default account".to_string())
                        .as_error();
                }
                Some(id) => id,
            },
        )
        .await?
    {
        GetOrCreateResponseKind::Created(guest_user) => {
            info!("Guest user created: {}", guest_user.email.get_email());
        }
        GetOrCreateResponseKind::NotCreated(_, msg) => {
            return use_case_err(format!("Guest user not created: {msg}"))
                .as_error()
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Notify guest user
    // ? -----------------------------------------------------------------------

    let mut parameters = vec![
        (
            "account_name",
            default_subscription_account.name.to_uppercase(),
        ),
        ("role_name", target_role.name.to_uppercase()),
        ("role_description", target_role.name),
        ("role_permissions", target_role.permission.to_string()),
    ];

    if let Some(description) = target_role.description {
        parameters.push(("role_description", description));
    }

    if let Err(err) = send_email_notification(
        parameters,
        "email/guest-to-subscription-account.jinja",
        life_cycle_settings,
        guest_email,
        None,
        String::from("[Guest to Account] You have been invited to collaborate"),
        message_sending_repo,
    )
    .await
    {
        return use_case_err(format!("Unable to send email: {err}"))
            .with_code(NativeErrorCodes::MYC00010)
            .as_error();
    };

    Ok(())
}