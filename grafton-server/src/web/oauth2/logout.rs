use axum_login::axum::routing::get;

use crate::core::AxumRouter;

pub fn router() -> AxumRouter {
    AxumRouter::new().route("/logout", get(self::get::logout))
}

mod get {
    use axum_login::axum::{
        http::StatusCode,
        response::{IntoResponse, Redirect},
    };

    use crate::AuthSession;

    pub async fn logout(mut auth_session: AuthSession) -> impl IntoResponse {
        match auth_session.logout() {
            Ok(_) => Redirect::to("/").into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
