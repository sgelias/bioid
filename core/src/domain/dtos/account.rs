use super::{guest::GuestUser, user::User};

use chrono::{DateTime, Local};
use clean_base::dtos::enums::{ChildrenEnum, ParentEnum};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountType {
    pub id: Option<Uuid>,

    pub name: String,
    pub description: String,

    pub is_subscription: bool,
    pub is_manager: bool,
    pub is_staff: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum AccountTypeEnum {
    Standard,
    Manager,
    Staff,
    Subscription,
}

impl Display for AccountTypeEnum {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            AccountTypeEnum::Standard => write!(f, "Standard"),
            AccountTypeEnum::Manager => write!(f, "Manager"),
            AccountTypeEnum::Staff => write!(f, "Staff"),
            AccountTypeEnum::Subscription => write!(f, "Subscription"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum VerboseProfileStatus {
    Pending,
    Active,
    Inactive,
    Archived,
    Unknown,
}

impl FromStr for VerboseProfileStatus {
    type Err = VerboseProfileStatus;

    fn from_str(s: &str) -> Result<VerboseProfileStatus, VerboseProfileStatus> {
        match s {
            "pending" => Ok(VerboseProfileStatus::Pending),
            "active" => Ok(VerboseProfileStatus::Active),
            "inactive" => Ok(VerboseProfileStatus::Inactive),
            "archived" => Ok(VerboseProfileStatus::Archived),
            _ => Err(VerboseProfileStatus::Unknown),
        }
    }
}

impl Display for VerboseProfileStatus {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            VerboseProfileStatus::Pending => write!(f, "pending"),
            VerboseProfileStatus::Active => write!(f, "active"),
            VerboseProfileStatus::Inactive => write!(f, "inactive"),
            VerboseProfileStatus::Archived => write!(f, "archived"),
            VerboseProfileStatus::Unknown => write!(f, "unknown"),
        }
    }
}

impl VerboseProfileStatus {
    pub fn from_profile(
        is_active: bool,
        is_checked: bool,
        is_archived: bool,
    ) -> Self {
        if is_active == false {
            return VerboseProfileStatus::Inactive;
        }

        if is_checked == false {
            return VerboseProfileStatus::Pending;
        }

        if is_archived == true {
            return VerboseProfileStatus::Archived;
        }

        if is_archived == false {
            return VerboseProfileStatus::Active;
        }

        VerboseProfileStatus::Unknown
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: Option<Uuid>,

    pub name: String,
    pub is_active: bool,
    pub is_checked: bool,
    pub is_archived: bool,
    pub verbose_status: Option<VerboseProfileStatus>,
    pub owner: ParentEnum<User, Uuid>,
    pub account_type: ParentEnum<AccountType, Uuid>,
    pub guest_users: Option<ChildrenEnum<GuestUser, Uuid>>,
    pub created: DateTime<Local>,
    pub updated: Option<DateTime<Local>>,
}

impl Account {
    pub fn build_owner_url(&self, base_url: String) -> Result<String, ()> {
        match self.owner.to_owned() {
            ParentEnum::Id(id) => Ok(format!("{:?}/{:?}", base_url, id)),
            ParentEnum::Record(record) => match record.id {
                None => Ok(base_url),
                Some(id) => Ok(format!("{}/{}", base_url, id.to_string())),
            },
        }
    }

    pub fn build_account_type_url(
        &self,
        base_url: String,
    ) -> Result<String, ()> {
        match self.account_type.to_owned() {
            ParentEnum::Id(id) => Ok(format!("{:?}/{:?}", base_url, id)),
            ParentEnum::Record(record) => match record.id {
                None => Ok(base_url),
                Some(id) => Ok(format!("{}/{}", base_url, id.to_string())),
            },
        }
    }

    pub fn build_guest_users_url(
        &self,
        base_url: String,
    ) -> Result<Vec<String>, ()> {
        match self.guest_users.to_owned() {
            None => Err(()),
            Some(records) => match records {
                ChildrenEnum::Ids(ids) => Ok(ids
                    .iter()
                    .map(|id| format!("{}/{}", base_url, id))
                    .collect()),
                ChildrenEnum::Records(records) => {
                    let urls = records
                        .iter()
                        .filter_map(|record| match record.id {
                            None => Some(base_url.to_owned()),
                            Some(_) => Some(format!(
                                "{}/{}",
                                base_url,
                                record.id.unwrap()
                            )),
                        })
                        .collect();

                    Ok(urls)
                }
            },
        }
    }
}

// ? --------------------------------------------------------------------------
// ? TESTS
// ? --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use chrono::Local;

    use super::*;
    use crate::domain::dtos::email::Email;

    #[test]
    fn test_if_account_works() {
        let base_url = "http://local.host/api/v1/accounts".to_string();

        let account_type = AccountType {
            id: None,
            name: "".to_string(),
            description: "".to_string(),
            is_subscription: false,
            is_manager: false,
            is_staff: false,
        };

        let user = User {
            id: None,
            username: "username".to_string(),
            email: Email::from_string("username@email.domain".to_string())
                .unwrap(),
            first_name: Some("first_name".to_string()),
            last_name: Some("last_name".to_string()),
            is_active: true,
            created: Local::now(),
            updated: Some(Local::now()),
        };

        let account = Account {
            id: None,
            name: String::from("Account Name"),
            is_active: true,
            is_checked: false,
            is_archived: false,
            verbose_status: None,
            owner: ParentEnum::Record(user),
            account_type: ParentEnum::Record(account_type),
            guest_users: None,
            created: Local::now(),
            updated: Some(Local::now()),
        };

        println!("{:?}", account.build_account_type_url(base_url.to_owned()));

        assert_eq!(
            account.build_account_type_url(base_url.to_owned()).is_ok(),
            true
        );

        assert_eq!(
            account.build_account_type_url(base_url.to_owned()).unwrap(),
            base_url.to_owned()
        );
    }
}
