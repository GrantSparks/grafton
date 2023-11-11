pub mod token;

pub mod user;
pub use user::{Role, User};

mod context;
pub use context::AppContext;

pub trait Identifiable<Id> {
    fn id(&self) -> Id;
}
