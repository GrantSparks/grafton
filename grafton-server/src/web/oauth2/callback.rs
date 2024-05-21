use crate::{
    core::AxumRouter,
    web::oauth2::{
        login::{OpenAiAuthParams, NEXT_URL_KEY},
        AuthzResp, Credentials, CSRF_STATE_KEY,
    },
    AuthSession, Config, Error, ServerConfigProvider,
};
use axum::{
    extract::{Form, Path, Query, State},
    response::{IntoResponse, Json, Redirect},
    routing::{get, post},
};
use axum_login::tower_sessions::Session;
use serde_json::json;
use tracing::{debug, error, info, warn};
use url::Url;

pub fn router<C>() -> AxumRouter<C>
where
    C: ServerConfigProvider,
{
    AxumRouter::new()
        .route("/oauth/:provider/callback", get(callback))
        .route("/oauth/token", post(get_access_token))
}

async fn get_access_token(
    mut auth_session: AuthSession,
    State(config): State<Config>,
    Form(OpenAiAuthParams {
        grant_type,
        client_id,
        client_secret,
        code,
        redirect_uri,
    }): Form<OpenAiAuthParams>,
) -> Result<impl IntoResponse, Error> {
    info!(
        "Received access token request with parameters: client_id={}, client_secret={}, grant_type={}, code={:?}, redirect_uri={}",
        client_id, client_secret, grant_type, code, redirect_uri
    );

    let provider = "github".to_string();
    let oauth_client = config
        .oauth_providers
        .get(&provider)
        .ok_or_else(|| Error::ProviderNotFoundError(provider.clone()))?;
    let userinfo_uri = oauth_client
        .extra
        .get("userinfo_uri")
        .and_then(|uri| uri.as_str())
        .ok_or_else(|| Error::ClientConfigNotFound("userinfo_uri".to_string()))?;

    let creds = Credentials {
        code: code.clone(),
        provider,
        userinfo_uri: userinfo_uri.to_string(),
    };

    let user = match auth_session.authenticate(creds).await {
        Ok(Some(user)) => {
            debug!("User authenticated successfully");
            user
        }
        Ok(None) => {
            warn!("Authentication succeeded but no user was found");
            return Err(Error::AuthenticationError("User not found".to_string()));
        }
        Err(e) => {
            error!("Internal error during authentication: {:?}", e);
            return Err(Error::AuthenticationError(e.to_string()));
        }
    };

    auth_session.login(&user).await.map_err(|e| {
        error!("Error logging in the user: {:?}", e);
        Error::LoginError("Error logging in the user".to_string())
    })?;

    let response_body = json!({
        "access_token": "example_token",
        "token_type": "bearer",
        "refresh_token": "example_token",
        "expires_in": 59,
    });

    Ok(Json(response_body))
}

async fn callback(
    session: Session,
    Path(provider): Path<String>,
    Query(AuthzResp {
        code,
        state: new_state,
    }): Query<AuthzResp>,
    State(config): State<Config>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    debug!("OAuth callback for provider: {}", provider);

    let old_state: Option<String> = session
        .get(CSRF_STATE_KEY)
        .await
        .map_err(|_| Error::SessionStateError("Failed to retrieve CSRF state".to_string()))?
        .ok_or(Error::MissingCSRFState)?;

    if old_state != Some(new_state.secret().to_string()) {
        return Err(Error::InvalidCSRFState);
    }

    match session.remove::<String>(NEXT_URL_KEY).await {
        Ok(Some(next)) if !next.is_empty() => {
            let mut url = Url::parse(&next).map_err(|_| {
                error!("Invalid URL in session: {}", next);
                Error::InvalidNextUrl(next)
            })?;
            url.query_pairs_mut().append_pair("code", &code);

            let url_str = url.to_string();
            Ok(Redirect::to(&url_str).into_response())
        }
        _ => Ok(Redirect::to(&config.website.pages.with_root().public_login).into_response()),
    }
}
