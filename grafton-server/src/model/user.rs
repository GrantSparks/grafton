use axum_login::AuthUser;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::Identifiable;

#[cfg(feature = "rbac")]
use oso::PolarClass;

#[cfg(feature = "rbac")]
#[derive(
    Debug, Default, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, PartialOrd, Copy, sqlx::Type,
)]
pub enum Role {
    #[default]
    None,
    User,
    Admin,
}

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
    pub access_token: String,
}

#[cfg(not(feature = "rbac"))]
#[derive(Debug, Default, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub pw_hash: Vec<u8>,
}

impl AuthUser for User {
    type Id = i64;

    fn session_auth_hash(&self) -> &[u8] {
        self.access_token.as_bytes()
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