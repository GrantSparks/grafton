use {oauth2::CsrfToken, serde::Deserialize};

mod logout;
pub(crate) use logout::router as create_logout_router;

mod callback;
pub(crate) use callback::router as create_callback_router;

mod login;
pub(crate) use login::router as create_login_router;

mod backend;
pub(crate) use backend::Backend;

pub(crate) const CSRF_STATE_KEY: &str = "oauth.csrf-state";

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct AuthzResp {
    code: String,
    state: CsrfToken,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Credentials {
    pub code: String,
    pub old_state: CsrfToken,
    pub new_state: CsrfToken,
    pub provider: String,
}

pub type AuthSession = axum_login::AuthSession<backend::Backend>;
