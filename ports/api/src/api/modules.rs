use myc_prisma::repositories::{
    AccountFetchingSqlDbRepository, AccountRegistrationSqlDbRepository,
    AccountTypeDeletionSqlDbRepository, AccountTypeRegistrationSqlDbRepository,
    AccountUpdatingSqlDbRepository, GuestRoleDeletionSqlDbRepository,
    GuestRoleFetchingSqlDbRepository, GuestRoleUpdatingSqlDbRepository,
    GuestUserRegistrationSqlDbRepository, ProfileFetchingSqlDbRepository,
    RoleDeletionSqlDbRepository, RoleFetchingSqlDbRepository,
    RoleRegistrationSqlDbRepository, UserRegistrationSqlDbRepository,
};
use myc_smtp::repositories::MessageSendingSqlDbRepository;
use shaku::module;

// ? ---------------------------------------------------------------------------
// ? Account
// ? ---------------------------------------------------------------------------

module! {
    pub AccountRegistrationModule {
        components = [AccountRegistrationSqlDbRepository],
        providers = []
    }
}

module! {
    pub AccountFetchingModule {
        components = [AccountFetchingSqlDbRepository],
        providers = []
    }
}

module! {
    pub AccountUpdatingModule {
        components = [AccountUpdatingSqlDbRepository],
        providers = []
    }
}

// ? ---------------------------------------------------------------------------
// ? Account Type
// ? ---------------------------------------------------------------------------

module! {
    pub AccountTypeRegistrationModule {
        components = [AccountTypeRegistrationSqlDbRepository],
        providers = []
    }
}

module! {
    pub AccountTypeDeletionModule {
        components = [AccountTypeDeletionSqlDbRepository],
        providers = []
    }
}

// ? ---------------------------------------------------------------------------
// ? Guest User
// ? ---------------------------------------------------------------------------

module! {
    pub GuestUserRegistrationModule {
        components = [GuestUserRegistrationSqlDbRepository],
        providers = []
    }
}

// ? ---------------------------------------------------------------------------
// ? Guest Role
// ? ---------------------------------------------------------------------------

module! {
    pub GuestRoleFetchingModule {
        components = [GuestRoleFetchingSqlDbRepository],
        providers = []
    }
}

module! {
    pub GuestRoleDeletionModule {
        components = [GuestRoleDeletionSqlDbRepository],
        providers = []
    }
}

module! {
    pub GuestRoleUpdatingModule {
        components = [GuestRoleUpdatingSqlDbRepository],
        providers = []
    }
}

// ? ---------------------------------------------------------------------------
// ? Message
// ? ---------------------------------------------------------------------------

module! {
    pub MessageSendingModule {
        components = [MessageSendingSqlDbRepository],
        providers = []
    }
}

// ? ---------------------------------------------------------------------------
// ? Profile
// ? ---------------------------------------------------------------------------

module! {
    pub ProfileFetchingModule {
        components = [ProfileFetchingSqlDbRepository],
        providers = []
    }
}

// ? ---------------------------------------------------------------------------
// ? Profile
// ? ---------------------------------------------------------------------------

module! {
    pub RoleRegistrationModule {
        components = [RoleRegistrationSqlDbRepository],
        providers = []
    }
}

module! {
    pub RoleFetchingModule {
        components = [RoleFetchingSqlDbRepository],
        providers = []
    }
}

module! {
    pub RoleDeletionModule {
        components = [RoleDeletionSqlDbRepository],
        providers = []
    }
}

// ? ---------------------------------------------------------------------------
// ? User
// ? ---------------------------------------------------------------------------

module! {
    pub UserRegistrationModule {
        components = [UserRegistrationSqlDbRepository],
        providers = []
    }
}