use crate::{axum::routing::get, core::AxumRouter};

pub fn router() -> AxumRouter {
    AxumRouter::new().route("/logout", get(self::get::logout))
}

mod get {

    use crate::{
        axum::{
            http::StatusCode,
            response::{IntoResponse, Redirect},
        },
        AuthSession,
    };

    pub async fn logout(mut auth_session: AuthSession) -> impl IntoResponse {
        match auth_session.logout().await {
            Ok(_) => Redirect::to("/").into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
