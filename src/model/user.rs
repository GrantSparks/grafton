use axum_login::AuthUser;
use oso::PolarClass;
use serde::{Deserialize, Serialize};

use super::Identifiable;

#[derive(Debug, Default, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, PartialOrd, Copy)]
pub enum Role {
    #[default]
    None,
    User,
    Admin,
}

impl PolarClass for Role {}

#[derive(Debug, Default, Clone, PolarClass, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct User {
    pub id: usize,
    pub username: String,
    #[polar(attribute)]
    pub role: Role,
    pub pw_hash: Vec<u8>,
}

impl AuthUser for User {
    type Id = usize;

    fn session_auth_hash(&self) -> &[u8] {
        &self.pw_hash
    }

    fn id(&self) -> Self::Id {
        Identifiable::id(self)
    }
}

impl Identifiable<usize> for User {
    fn id(&self) -> usize {
        self.id
    }
}
