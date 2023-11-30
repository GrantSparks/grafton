use std::sync::Arc;

use axum_login::axum::routing::get;

use crate::model::AppContext;

pub fn router() -> axum_login::axum::Router<Arc<AppContext>> {
    axum_login::axum::Router::new().route("/logout", get(self::get::logout))
}

mod get {
    use axum_login::axum::{
        http::StatusCode,
        response::{IntoResponse, Redirect},
    };

    use crate::web::oauth2::AuthSession;

    pub async fn logout(mut auth_session: AuthSession) -> impl IntoResponse {
        match auth_session.logout() {
            Ok(_) => Redirect::to("/login/github").into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
