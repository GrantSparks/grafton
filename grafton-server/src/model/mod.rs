mod user;
#[cfg(feature = "rbac")]
pub use user::Role;
pub use user::User;

mod context;
pub use context::Context;

pub trait Identifiable<Id> {
    fn id(&self) -> Id;
}
