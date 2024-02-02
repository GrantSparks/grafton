use {
    oxide_auth::{
        endpoint::WebRequest,
        frontends::simple::endpoint::Vacant,
        primitives::{
            authorizer::AuthMap, generator::RandomGenerator, issuer::TokenMap, prelude::*,
        },
    },
    typed_builder::TypedBuilder,
};

use crate::{Config, Error};

#[derive(TypedBuilder)]
#[allow(dead_code)]
pub struct AccessTokenEndpointBuilder<R: WebRequest> {
    #[builder(default = None, setter(skip))]
    authorizer: Option<AuthMap<RandomGenerator>>,
    #[builder(default = None, setter(skip))]
    registrar: Option<ClientMap>,
    #[builder(default = None, setter(skip))]
    issuer: Option<TokenMap<RandomGenerator>>,
    #[builder(default = None, setter(skip))]
    scopes: Option<Vacant>,
    #[builder(default = None, setter(skip))]
    solicitor: Option<Vacant>,
    #[builder(default = None, setter(skip))]
    response: Option<Vacant>,
    #[builder(default, setter(skip))]
    _marker: std::marker::PhantomData<R>,
}

#[allow(dead_code)]
impl<R: WebRequest> AccessTokenEndpointBuilder<R>
where
    R::Response: Default,
{
    pub fn with_registrar_from_config(mut self, config: &Config) -> Result<Self, Error> {
        let client_map: ClientMap = config.clone().try_into()?;
        self.registrar = Some(client_map);
        Ok(self)
    }
}

// Routes
// /oauth/auth: The authorization endpoint
// /oauth/token: The access token endpoint
// /oauth/refresh: The refresh token endpoint
