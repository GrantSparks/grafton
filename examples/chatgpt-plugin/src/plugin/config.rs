use std::sync::Arc;

use grafton_auth::{AuthConfigProvider, Config as AuthConfig};

use crate::AppContext;

use {
    derivative::Derivative,
    grafton_config::TokenExpandingConfig,
    grafton_server::{axum::extract::FromRef, Config as ServerConfig, ServerConfigProvider},
    openapiv3::OpenAPI,
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Serialize, Deserialize, Derivative, Clone)]
#[derivative(Default)]
#[serde(default)]
pub struct VerificationTokens {
    pub openai: String,
}

#[derive(Debug, Serialize, Deserialize, Derivative, Clone)]
#[derivative(Default)]
#[serde(default)]
pub struct AuthInfo {
    #[serde(rename = "type")]
    pub auth_type: String,
    pub client_url: String,
    pub scope: String,
    pub authorization_url: String,
    pub authorization_content_type: String,
    pub verification_tokens: VerificationTokens,
}

#[derive(Debug, Serialize, Deserialize, Derivative, Clone)]
#[derivative(Default)]
#[serde(default)]
pub struct ApiInfo {
    #[serde(rename = "type")]
    pub api_type: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Derivative, Clone)]
#[derivative(Default)]
#[serde(default)]
pub struct Info {
    pub schema_version: String,
    pub name_for_human: String,
    pub name_for_model: String,
    pub description_for_human: String,
    pub description_for_model: String,
    pub auth: AuthInfo,
    pub api: ApiInfo,
    pub logo_url: String,
    pub contact_email: String,
    pub legal_info_url: String,
}

#[derive(Debug, Serialize, Deserialize, Derivative, Clone)]
#[derivative(Default)]
#[serde(default)]
pub struct ChatGptPlugin {
    pub plugin_info: Info,
    #[derivative(Default(value = "\"/.well-known/ai-plugin.json\".into()"))]
    pub plugin_json: String,
    #[derivative(Default(value = "\"/chatgpt-plugin/openapi.yaml\".into()"))]
    pub openapi_yaml: String,
    pub openapi: OpenAPI,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(flatten)]
    pub base: AuthConfig,
    pub chatgpt_plugin: ChatGptPlugin,
}

impl ServerConfigProvider for Config {
    fn get_server_config(&self) -> &ServerConfig {
        self.base.get_server_config()
    }
}

impl AuthConfigProvider for Config {
    fn get_auth_config(&self) -> &AuthConfig {
        &self.base
    }
}

impl TokenExpandingConfig for Config {}

impl FromRef<Arc<AppContext>> for ChatGptPlugin {
    fn from_ref(state: &Arc<AppContext>) -> Self {
        state.config.chatgpt_plugin.clone()
    }
}
