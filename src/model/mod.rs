mod token;
pub use token::Token;

mod user;
#[cfg(feature = "rbac")]
pub use user::Role;
pub use user::User;

mod context;
pub use context::AppContext;

pub trait Identifiable<Id> {
    fn id(&self) -> Id;
}
