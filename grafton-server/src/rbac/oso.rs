use oso::{Class, Oso, PolarClass};

use crate::{
    error::AppError,
    model::{Role, User},
    util::Config,
};

pub fn initialize_oso(config: &Config) -> Result<Oso, AppError> {
    let mut oso = Oso::new();

    oso.register_class(Class::builder::<Role>().build())?;

    oso.register_class(
        User::get_polar_class_builder()
            .add_method("is_none", |u: &User| matches!(u.role, Role::None))
            .add_method("is_user", |u: &User| matches!(u.role, Role::User))
            .add_method("is_admin", |u: &User| matches!(u.role, Role::Admin))
            .build(),
    )?;

    oso.load_files(config.oso_policy_files.clone())?;

    Ok(oso)
}
