use {
    axum_login::AuthUser,
    serde::{Deserialize, Serialize},
    sqlx::FromRow,
};

#[cfg(feature = "rbac")]
use strum::{Display, EnumString, EnumVariantNames};

use crate::new_secret_type;

use super::Identifiable;

#[cfg(feature = "rbac")]
use oso::PolarClass;

#[cfg(feature = "rbac")]
#[derive(
    Default,
    Display,
    EnumString,
    EnumVariantNames,
    Debug,
    Serialize,
    Deserialize,
    Clone,
    Eq,
    PartialEq,
    Hash,
    PartialOrd,
    Copy,
    sqlx::Type,
)]
#[strum(serialize_all = "snake_case")]
pub enum Role {
    #[default]
    None,
    User,
    Admin,
}

new_secret_type![
    /// AccessToken wraps the token to prevent accidental leakage.
    #[derive(Default, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, sqlx::Type)]
    #[sqlx(transparent)]
    AccessToken(String)
];

#[cfg(feature = "rbac")]
impl PolarClass for Role {}

#[cfg(feature = "rbac")]
#[derive(
    Debug, Default, Clone, PolarClass, Serialize, Deserialize, Eq, PartialEq, Hash, FromRow,
)]
pub struct User {
    pub id: i64,
    pub username: String,
    #[polar(attribute)]
    pub role: Role,
    pub access_token: AccessToken,
}

#[cfg(not(feature = "rbac"))]
#[derive(Debug, Default, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub access_token: AccessToken,
}

impl AuthUser for User {
    type Id = i64;

    fn session_auth_hash(&self) -> &[u8] {
        self.access_token.secret().as_bytes() // TODO: improve less than ideal hash function
    }

    fn id(&self) -> Self::Id {
        Identifiable::id(self)
    }
}

impl Identifiable<i64> for User {
    fn id(&self) -> i64 {
        self.id
    }
}
