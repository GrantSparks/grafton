use {oauth2::CsrfToken, serde::Deserialize};

mod logout;
pub use logout::router as create_logout_router;

mod callback;
pub use callback::router as create_callback_router;

mod login;
pub use login::router as create_login_router;

pub mod backend;

pub const CSRF_STATE_KEY: &str = "oauth.csrf-state";

#[derive(Debug, Clone, Deserialize)]
pub struct AuthzResp {
    code: String,
    state: CsrfToken,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthzReq {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub state: CsrfToken,
    pub scope: String,
}

impl std::fmt::Display for AuthzReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "response_type={}&client_id={}&redirect_uri={}&state={:?}&scope={}",
            self.response_type, self.client_id, self.redirect_uri, self.state, self.scope
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    pub code: String,
    pub provider: String,
    pub userinfo_uri: String,
}
