mod user;
#[cfg(feature = "rbac")]
pub use user::Role;
pub use user::User;

mod context;
pub use context::AppContext;

mod types;
pub use types::*;

pub trait Identifiable<Id> {
    fn id(&self) -> Id;
}
