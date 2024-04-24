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
pub struct Credentials {
    pub code: String,
    pub old_state: CsrfToken,
    pub new_state: CsrfToken,
    pub provider: String,
    pub userinfo_uri: String,
}
