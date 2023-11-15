mod oso;

use ::oso::Oso;
use oso::initialize_oso;

use crate::{error::AppError, util::Config};

pub fn initialize(config: &Config) -> Result<Oso, AppError> {
    initialize_oso(config)
}
