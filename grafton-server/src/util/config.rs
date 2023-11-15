use anyhow::Result;
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{collections::HashMap, env, fmt, net::IpAddr, sync::Arc};
use url::Url;

use crate::util::token_expander::expand_tokens;

pub fn load_config(config_dir: &str) -> Result<Arc<Config>> {
    let config = Config::load(config_dir)?;
    Ok(Arc::new(config))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SessionConfig {
    pub same_site_policy: SameSiteConfig,
}

impl Default for SessionConfig {
    fn default() -> Self {
        SessionConfig {
            same_site_policy: SameSiteConfig::Lax,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct LoggerConfig {
    pub verbosity: Verbosity,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        LoggerConfig {
            verbosity: Verbosity::Info,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Verbosity {
    Trace,
    Info,
    Debug,
    Warn,
    Error,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Routes {
    base: String,
    public_home: String,
    public_error: String,
}

impl Default for Routes {
    fn default() -> Self {
        Routes {
            base: "/".to_string(),
            public_home: "".to_string(),
            public_error: "error".to_string(),
        }
    }
}

impl Routes {
    pub fn with_base_prepend(&self) -> Self {
        let normalized_base = self.normalize_slash(&self.base);
        Self {
            base: normalized_base.clone(),
            public_home: self.join_paths(&normalized_base, &self.public_home),
            public_error: self.join_paths(&normalized_base, &self.public_error),
        }
    }

    fn normalize_slash(&self, path: &str) -> String {
        if !path.ends_with('/') {
            format!("{}/", path)
        } else {
            path.to_string()
        }
    }

    fn join_paths(&self, base: &str, path: &str) -> String {
        let trimmed_base = base.trim_end_matches('/');
        let trimmed_path = path.trim_start_matches('/');
        format!("{}/{}", trimmed_base, trimmed_path)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Website {
    pub web_root: String,
    pub index_page: String,
    pub bind_ssl_config: SslConfig,
    pub bind_address: std::net::IpAddr,
    pub bind_ports: Ports,
    pub public_hostname: String,
    pub public_ports: Ports,
    pub public_ssl_enabled: bool,
    pub oso_policy_files: Vec<String>,
}

impl Website {
    pub fn public_server_url(&self) -> String {
        let (protocol, port) = self.get_protocol_and_port();
        self.format_url(protocol, port)
    }

    pub fn format_public_server_url(&self, path: &str) -> String {
        let url = self.public_server_url();
        format!(
            "{}/{}",
            url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    fn get_protocol_and_port(&self) -> (&str, u16) {
        if self.public_ssl_enabled {
            ("https", self.public_ports.https)
        } else {
            ("http", self.public_ports.http)
        }
    }

    fn is_default_port(protocol: &str, port: u16) -> bool {
        matches!((protocol, port), ("http", 80) | ("https", 443))
    }

    fn format_hostname_and_port(&self, protocol: &str, port: u16) -> String {
        if Self::is_default_port(protocol, port) {
            format!("{}://{}", protocol, self.public_hostname)
        } else {
            format!("{}://{}:{}", protocol, self.public_hostname, port)
        }
    }

    fn format_url(&self, protocol: &str, port: u16) -> String {
        let base = format!("{}://{}", protocol, self.public_hostname);
        let mut url = Url::parse(&base).expect("Invalid base URL");

        if !Self::is_default_port(protocol, port) {
            url.set_port(Some(port)).expect("Invalid port");
        }
        url.to_string().trim_end_matches('/').to_string()
    }
}

impl Default for Website {
    fn default() -> Self {
        Website {
            public_hostname: "localhost".into(),
            public_ports: Ports::default(),
            public_ssl_enabled: false,
            bind_address: "127.0.0.1".parse().expect("Invalid IP address"),
            bind_ssl_config: SslConfig::default(),
            index_page: "index.html".into(),
            web_root: "../public/www".into(),
            bind_ports: Ports::default(),
            oso_policy_files: vec![],
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum SameSiteConfig {
    Strict,
    Lax,
    None,
}

impl fmt::Display for SameSiteConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SameSiteConfig::Strict => write!(f, "Strict"),
            SameSiteConfig::Lax => write!(f, "Lax"),
            SameSiteConfig::None => write!(f, "None"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SslConfig {
    pub enabled: bool,
    pub cert_path: String,
    pub key_path: String,
}

impl Default for SslConfig {
    fn default() -> Self {
        SslConfig {
            enabled: false,
            cert_path: "config/cert.pem".to_string(),
            key_path: "config/key.pem".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Ports {
    pub http: u16,
    pub https: u16,
    pub grpc: u16,
}

impl Default for Ports {
    fn default() -> Self {
        Ports {
            http: 80,
            https: 443,
            grpc: 9339,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientConfig {
    pub client_id: String,
    pub client_secret: String,
    pub auth_uri: String,
    pub token_uri: String,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    #[serde(default = "default_run_mode")]
    pub run_mode: String,
    #[serde(default)]
    pub logger: LoggerConfig,
    #[serde(default)]
    pub website: Website,
    #[serde(default)]
    pub session: SessionConfig,
    #[serde(default)]
    pub routes: Routes,
    pub oauth_clients: HashMap<String, ClientConfig>,
}

fn default_run_mode() -> String {
    "dev".to_string()
}

impl Config {
    pub fn load(config_dir: &str) -> Result<Self> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "dev".to_string());
        let figment = Figment::from(Config::figment_with_paths(config_dir, &run_mode));

        let config: Config = figment.extract()?;
        let config_value: Value = serde_json::to_value(&config)?;
        let replaced = expand_tokens(&config_value);
        serde_json::from_value(replaced).map_err(Into::into)
    }

    pub fn figment_with_paths(config_dir: &str, run_mode: &str) -> Figment {
        let original_env = env::vars().collect::<Vec<_>>();
        for (key, value) in &original_env {
            let new_key = Config::map_env_var(key.to_string());
            env::set_var(new_key, value);
        }

        let current_dir = env::current_dir().expect("Failed to get current directory");
        let absolute_config_dir = current_dir.join(config_dir);
        let default_path = absolute_config_dir.join("default.toml");
        let local_path = absolute_config_dir.join("local.toml");
        let run_mode_path = absolute_config_dir.join(format!("{}.toml", run_mode));

        let mut figment = Figment::new().merge(Toml::file(default_path));

        if local_path.exists() {
            figment = figment.merge(Toml::file(local_path));
        }
        if run_mode_path.exists() {
            figment = figment.merge(Toml::file(run_mode_path));
        }

        figment = figment.merge(Env::raw());

        for (key, value) in original_env {
            env::set_var(key, value);
        }

        figment
    }

    fn map_env_var(key: String) -> String {
        key.as_str()
            .replace("WEBSITE_", "WEBSITE.")
            .replace("SESSION_", "SESSION.")
            .replace("LOGGER_", "LOGGER.")
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_prepend() {
        let routes = Routes {
            base: "/api".to_string(),
            public_home: "home".to_string(),
            public_error: "/error".to_string(),
        };

        let updated_routes = routes.with_base_prepend();
        assert_eq!(updated_routes.public_home, "/api/home");
        assert_eq!(updated_routes.public_error, "/api/error");
    }

    #[test]
    fn test_base_prepend_with_trailing_slash() {
        let routes = Routes {
            base: "/api/".to_string(),
            public_home: "home".to_string(),
            public_error: "error".to_string(),
        };

        let updated_routes = routes.with_base_prepend();
        assert_eq!(updated_routes.public_home, "/api/home");
        assert_eq!(updated_routes.public_error, "/api/error");
    }

    #[test]
    fn test_base_prepend_with_empty_and_root_routes() {
        let routes = Routes {
            base: "/".to_string(),
            public_home: "".to_string(),
            public_error: "/".to_string(),
        };

        let updated_routes = routes.with_base_prepend();
        assert_eq!(updated_routes.public_home, "/");
        assert_eq!(updated_routes.public_error, "/");
    }

    fn create_website(
        ssl_enabled: bool,
        http_port: u16,
        https_port: u16,
        hostname: &str,
    ) -> Website {
        Website {
            public_ssl_enabled: ssl_enabled,
            public_ports: Ports {
                http: http_port,
                https: https_port,
                grpc: Default::default(),
            },
            public_hostname: hostname.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn public_server_url_with_ssl_default_port() {
        let website = create_website(true, 80, 443, "example.com");
        assert_eq!(website.public_server_url(), "https://example.com");
    }

    #[test]
    fn public_server_url_with_ssl_non_default_port() {
        let website = create_website(true, 80, 8443, "example.com");
        assert_eq!(website.public_server_url(), "https://example.com:8443");
    }

    #[test]
    fn public_server_url_without_ssl_default_port() {
        let website = create_website(false, 80, 443, "example.com");
        assert_eq!(website.public_server_url(), "http://example.com");
    }

    #[test]
    fn public_server_url_without_ssl_non_default_port() {
        let website = create_website(false, 8080, 443, "example.com");
        assert_eq!(website.public_server_url(), "http://example.com:8080");
    }

    #[test]
    fn format_public_server_url_root_path() {
        let website = create_website(true, 80, 443, "example.com");
        assert_eq!(
            website.format_public_server_url("/"),
            "https://example.com/"
        );
    }

    #[test]
    fn format_public_server_url_sub_path() {
        let website = create_website(false, 8080, 443, "example.com");
        assert_eq!(
            website.format_public_server_url("/api"),
            "http://example.com:8080/api"
        );
    }

    #[test]
    fn test_routes_deserialization() {
        let json = r#"{
            "base": "/api",
            "public_home": "/home",
            "public_error": "/error"
        }"#;

        let deserialized: Routes = serde_json::from_str(json).expect("Deserialization failed");
        assert_eq!(deserialized.base, "/api");
        assert_eq!(deserialized.public_home, "/home");
        assert_eq!(deserialized.public_error, "/error");
    }

    #[test]
    fn test_normalize_slash() {
        let routes = Routes::default();
        assert_eq!(routes.normalize_slash("path"), "path/");
        assert_eq!(routes.normalize_slash("path/"), "path/");
    }

    #[test]
    fn test_join_paths() {
        let routes = Routes::default();
        assert_eq!(routes.join_paths("/base", "/path"), "/base/path");
        assert_eq!(routes.join_paths("/base/", "/path"), "/base/path");
        assert_eq!(routes.join_paths("/base", "path"), "/base/path");
        assert_eq!(routes.join_paths("/base/", "path"), "/base/path");
        assert_eq!(routes.join_paths("/base/", "//path"), "/base/path");
    }

    #[test]
    fn test_with_base_prepend() {
        let mut routes = Routes::default();
        routes.base = "/api".to_string();
        routes.public_home = "home".to_string();
        routes.public_error = "/error".to_string();

        let new_routes = routes.with_base_prepend();
        assert_eq!(new_routes.base, "/api/");
        assert_eq!(new_routes.public_home, "/api/home");
        assert_eq!(new_routes.public_error, "/api/error");
    }

    #[test]
    fn test_load_single_oauth_client() {
        let json = r#"
        {
            "run_mode": "test",
            "oauth_clients": {
                "github": {
                    "client_id": "github_id",
                    "client_secret": "github_secret",
                    "redirect_uri": "http://localhost/redirect",
                    "auth_uri": "http://localhost/callback",
                    "token_uri": "http://localhost/token"
                }
            }
        }"#;

        let config: Config = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(
            config.oauth_clients.get("github").unwrap().client_id,
            "github_id"
        );
        assert_eq!(
            config.oauth_clients.get("github").unwrap().client_secret,
            "github_secret"
        );
        assert_eq!(
            config.oauth_clients.get("github").unwrap().auth_uri,
            "http://localhost/callback"
        );
        assert_eq!(
            config.oauth_clients.get("github").unwrap().token_uri,
            "http://localhost/token"
        );
    }

    #[test]
    fn test_load_multiple_oauth_clients() {
        let json = r#"
        {
            "run_mode": "test",
            "oauth_clients": {
                "github": {
                    "client_id": "github_id",
                    "client_secret": "github_secret",
                    "redirect_uri": "http://localhost/github/redirect",
                    "auth_uri": "http://localhost/github/callback",
                    "token_uri": "http://localhost/github/token"
                },
                "google": {
                    "client_id": "google_id",
                    "client_secret": "google_secret",
                    "redirect_uri": "http://localhost/google/redirect",
                    "auth_uri": "http://localhost/google/callback",
                    "token_uri": "http://localhost/google/token"
                }
            }
        }"#;

        let config: Config = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(
            config.oauth_clients.get("github").unwrap().token_uri,
            "http://localhost/github/token"
        );
        assert_eq!(
            config.oauth_clients.get("google").unwrap().token_uri,
            "http://localhost/google/token"
        );
    }

    #[test]
    fn test_deserialization_error_for_missing_fields_with_token_uri() {
        let json = r#"
        {
            "run_mode": "test",
            "oauth_clients": {
                "invalid_client": {
                    "client_id": "id_without_secret",
                    // missing fields
                }
            }
        }"#;

        let result: Result<Config, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
