use std::{
    collections::HashMap,
    env,
    net::IpAddr,
    path::{Path, PathBuf},
};

use anyhow::Result;
use derivative::Derivative;
use figment::{
    providers::{Format, Toml},
    Figment,
};
use oauth2::{ClientId, ClientSecret};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use strum::{Display, EnumString, EnumVariantNames};
use url::Url;

use crate::{util::token_expander::expand_tokens, AppError};

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SessionConfig {
    pub same_site_policy: SameSiteConfig,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct LoggerConfig {
    pub verbosity: Verbosity,
}

#[derive(
    Default, EnumString, EnumVariantNames, Debug, Serialize, Deserialize, Clone, PartialEq,
)]
#[strum(serialize_all = "snake_case")]
pub enum Verbosity {
    Trace,
    #[default]
    Info,
    Debug,
    Warn,
    Error,
}

#[derive(Debug, Serialize, Deserialize, Derivative, Clone)]
#[derivative(Default)]
#[serde(default)]
pub struct Pages {
    #[derivative(Default(value = "\"/\".into()"))]
    pub root: String,
    #[derivative(Default(value = "\"\".into()"))]
    pub public_home: String,
    #[derivative(Default(value = "\"error\".into()"))]
    pub public_error: String,
    #[derivative(Default(value = "\"login\".into()"))]
    pub public_login: String,
}

impl Pages {
    /// Returns a new `Pages` struct with the `root` path prepended to all paths.
    pub fn with_root(&self) -> Self {
        let normalized_base = self.normalize_slash(&self.root);
        Self {
            root: normalized_base.clone(),
            public_home: self.join_paths(&normalized_base, &self.public_home),
            public_error: self.join_paths(&normalized_base, &self.public_error),
            public_login: self.join_paths(&normalized_base, &self.public_login),
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

#[derive(Debug, Serialize, Deserialize, Derivative, Clone)]
#[derivative(Default)]
#[serde(default)]
pub struct Website {
    #[derivative(Default(value = "\"../public/www\".into()"))]
    pub web_root: String,

    #[derivative(Default(value = "\"index.html\".into()"))]
    pub index_page: String,

    #[derivative(Default)]
    pub bind_ssl_config: SslConfig,

    #[derivative(Default(value = "\"127.0.0.1\".parse().unwrap()"))]
    pub bind_address: IpAddr,

    #[derivative(Default)]
    pub bind_ports: Ports,

    #[derivative(Default(value = "\"localhost\".into()"))]
    pub public_hostname: String,

    #[derivative(Default)]
    pub public_ports: Ports,

    #[derivative(Default(value = "false"))]
    pub public_ssl_enabled: bool,

    #[derivative(Default)]
    #[serde(default)]
    pub pages: Pages,
}

impl Website {
    pub fn public_server_url(&self) -> String {
        let (protocol, port) = self.get_protocol_and_port();
        match self.format_url(protocol, port) {
            Ok(url) => url,
            Err(err) => {
                // Handle the error, log it, or return a default/fallback URL
                // For example:
                eprintln!("Error generating URL: {}", err);
                String::new() // or a default/fallback URL
            }
        }
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

    #[allow(unused)]
    fn format_hostname_and_port(&self, protocol: &str, port: u16) -> String {
        if Self::is_default_port(protocol, port) {
            format!("{}://{}", protocol, self.public_hostname)
        } else {
            format!("{}://{}:{}", protocol, self.public_hostname, port)
        }
    }

    fn format_url(&self, protocol: &str, port: u16) -> Result<String, AppError> {
        let base = format!("{}://{}", protocol, self.public_hostname);
        let mut url = Url::parse(&base).map_err(|e| AppError::InvalidAuthUrl(e.to_string()))?;

        if !Self::is_default_port(protocol, port) {
            url.set_port(Some(port))
                .map_err(|_| AppError::InvalidAuthUrl("Invalid port".to_string()))?;
        }

        Ok(url.to_string().trim_end_matches('/').to_string())
    }
}

#[derive(
    Default, Display, EnumString, EnumVariantNames, Debug, Serialize, Deserialize, Clone, PartialEq,
)]
#[strum(serialize_all = "snake_case")]
pub enum SameSiteConfig {
    Strict,
    #[default]
    Lax,
    None,
}

#[derive(Debug, Serialize, Deserialize, Derivative, Clone)]
#[derivative(Default)]
#[serde(default)]
pub struct SslConfig {
    #[derivative(Default(value = "false"))]
    pub enabled: bool,
    #[derivative(Default(value = "\"config/cert.pem\".into()"))]
    pub cert_path: String,
    #[derivative(Default(value = "\"config/key.pem\".into()"))]
    pub key_path: String,
}

#[derive(Debug, Serialize, Deserialize, Derivative, Clone)]
#[derivative(Default)]
#[serde(default)]
pub struct Ports {
    #[derivative(Default(value = "80"))]
    pub http: u16,
    #[derivative(Default(value = "443"))]
    pub https: u16,
    #[derivative(Default(value = "9339"))]
    pub grpc: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientConfig {
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
    pub auth_uri: String,
    pub token_uri: String,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Derivative, Clone)]
#[derivative(Default)]
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
    pub oauth_clients: HashMap<String, ClientConfig>,
    #[derivative(Default(value = "Vec::new()"))]
    pub oso_policy_files: Vec<String>,
}

fn default_run_mode() -> String {
    "dev".to_string()
}

impl Config {
    pub(crate) fn load(config_dir: &str) -> Result<Self> {
        let run_mode = determine_run_mode();
        let config_paths = setup_config_paths(config_dir, &run_mode);

        let mut figment = Figment::new();
        for path in config_paths {
            if path.exists() {
                let config = load_config_from_file(&path)?;
                figment = figment.merge(config);
            } else {
                println!("Configuration file not found: {:?}", path);
            }
        }
        handle_env_vars();
        let config: Config = figment.extract()?;
        let config_value: Value = serde_json::to_value(config)?;
        let replaced = expand_tokens(&config_value);
        serde_json::from_value(replaced).map_err(Into::into)
    }
}

fn determine_run_mode() -> String {
    env::var("RUN_MODE").unwrap_or_else(|_| "dev".to_string())
}

fn setup_config_paths(config_dir: &str, run_mode: &str) -> Vec<PathBuf> {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let absolute_config_dir = current_dir.join(config_dir);

    vec![
        absolute_config_dir.join("default.toml"),
        absolute_config_dir.join("local.toml"),
        absolute_config_dir.join(format!("{}.toml", run_mode)),
    ]
}

fn load_config_from_file(path: &Path) -> Result<Figment, AppError> {
    if path.exists() {
        Ok(Figment::new().merge(Toml::file(path)))
    } else {
        Err(AppError::ConfigError(format!("File not found: {:?}", path)))
    }
}

fn handle_env_vars() {
    let original_env = env::vars().collect::<Vec<_>>();
    for (key, value) in &original_env {
        let new_key = map_env_var(key);
        env::set_var(new_key, value);
    }
}

fn map_env_var(key: &str) -> String {
    // Example transformation logic; adjust as needed for your application
    key.replace("WEBSITE_", "WEBSITE.")
        .replace("SESSION_", "SESSION.")
        .replace("LOGGER_", "LOGGER.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_prepend() -> Result<(), AppError> {
        let pages = Pages {
            root: "/api".to_string(),
            public_home: "home".to_string(),
            public_error: "/error".to_string(),
            public_login: "login".to_string(),
        };

        let updated_pages = pages.with_root();
        assert_eq!(updated_pages.public_home, "/api/home");
        assert_eq!(updated_pages.public_error, "/api/error");
        assert_eq!(updated_pages.public_login, "/api/login");
        Ok(())
    }

    #[test]
    fn test_base_prepend_with_trailing_slash() {
        let pages = Pages {
            root: "/api/".to_string(),
            public_home: "home".to_string(),
            public_error: "error".to_string(),
            public_login: "login".to_string(),
        };

        let updated_pages = pages.with_root();
        assert_eq!(updated_pages.public_home, "/api/home");
        assert_eq!(updated_pages.public_error, "/api/error");
        assert_eq!(updated_pages.public_login, "/api/login");
    }

    #[test]
    fn test_base_prepend_with_empty_and_root_pages() {
        let pages = Pages {
            root: "/".to_string(),
            public_home: "".to_string(),
            public_error: "/".to_string(),
            public_login: "/".to_string(),
        };

        let updated_pages = pages.with_root();
        assert_eq!(updated_pages.public_home, "/");
        assert_eq!(updated_pages.public_error, "/");
        assert_eq!(updated_pages.public_login, "/");
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
    fn test_pages_deserialization() {
        let json = r#"{
            "root": "/api",
            "public_home": "/home",
            "public_error": "/error"
        }"#;

        let deserialized: Pages = serde_json::from_str(json).expect("Deserialization failed");
        assert_eq!(deserialized.root, "/api");
        assert_eq!(deserialized.public_home, "/home");
        assert_eq!(deserialized.public_error, "/error");
    }

    #[test]
    fn test_normalize_slash() {
        let pages = Pages::default();
        assert_eq!(pages.normalize_slash("path"), "path/");
        assert_eq!(pages.normalize_slash("path/"), "path/");
    }

    #[test]
    fn test_join_paths() {
        let pages = Pages::default();
        assert_eq!(pages.join_paths("/root", "/path"), "/root/path");
        assert_eq!(pages.join_paths("/root/", "/path"), "/root/path");
        assert_eq!(pages.join_paths("/root", "path"), "/root/path");
        assert_eq!(pages.join_paths("/root/", "path"), "/root/path");
        assert_eq!(pages.join_paths("/root/", "//path"), "/root/path");
    }

    #[test]
    fn test_with_base_prepend() {
        let pages = Pages {
            root: "/api".to_string(),
            public_home: "home".to_string(),
            public_error: "/error".to_string(),
            public_login: "login".to_string(),
        };

        let new_pages = pages.with_root();
        assert_eq!(new_pages.root, "/api/");
        assert_eq!(new_pages.public_home, "/api/home");
        assert_eq!(new_pages.public_error, "/api/error");
        assert_eq!(new_pages.public_login, "/api/login");
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
            ClientId::new("github_id".to_string())
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

    #[test]
    fn test_format_hostname_and_port_default_http_port() {
        let website = Website {
            public_hostname: "example.com".into(),
            public_ports: Ports::default(), // HTTP port 80
            public_ssl_enabled: false,
            ..Default::default()
        };
        assert_eq!(
            website.format_hostname_and_port("http", website.public_ports.http),
            "http://example.com"
        );
    }

    #[test]
    fn test_format_hostname_and_port_non_default_https_port() {
        let website = Website {
            public_hostname: "example.com".into(),
            public_ports: Ports {
                https: 8443,
                ..Default::default()
            },
            public_ssl_enabled: true,
            ..Default::default()
        };
        assert_eq!(
            website.format_hostname_and_port("https", website.public_ports.https),
            "https://example.com:8443"
        );
    }

    #[test]
    fn test_config_load_with_local_override() {
        let config_toml_content = r#"
            run_mode = "dev"
    
            [website]
            bind_ports = { http = 8080 }
            public_ports = { http = 8080 }
    
            [session]
    
            [pages]
    
            [oauth_clients]
            google = { client_id = "YOUR GOOGLE CLIENT ID", client_secret = "YOUR GOOGLE CLIENT SECRET", auth_uri = "", token_uri = "" }
            github = { client_id = "xxx", client_secret = "xxx", auth_uri = "https://github.com/login/oauth/authorize", token_uri = "https://github.com/login/oauth/access_token" }
        "#;

        let local_toml_content = r#"
            [logger]
            verbosity = "Debug"
    
            [website]
            bind_ssl_config = { enabled = false }
            public_ports = { http = 80, https = 443 }
            public_ssl_enabled = true
        "#;

        // Create a temporary directory
        let temp_dir = tempfile::tempdir().expect("Failed to create a temporary directory");
        let config_dir = temp_dir.path();

        // Simulate loading and merging of TOML files
        let default_path = config_dir.join("default.toml");
        let local_path = config_dir.join("local.toml");

        std::fs::write(default_path, config_toml_content)
            .expect("Failed to write to temp default.toml file");
        std::fs::write(local_path, local_toml_content)
            .expect("Failed to write to temp local.toml file");
        let loaded_config_after_local_toml = Config::load(config_dir.to_str().unwrap())
            .expect("Failed to load config after local.toml");

        // Assertions to verify the loaded configuration
        assert_eq!(loaded_config_after_local_toml.run_mode, "dev");
        assert_eq!(loaded_config_after_local_toml.website.bind_ports.http, 8080);
        assert_eq!(loaded_config_after_local_toml.website.public_ports.http, 80);
        assert_eq!(
            loaded_config_after_local_toml.website.public_ports.https,
            443
        );
        assert!(loaded_config_after_local_toml.website.public_ssl_enabled);
        assert_eq!(
            loaded_config_after_local_toml.logger.verbosity,
            Verbosity::Debug
        );

        // Check for existence of OAuth clients and then assert
        if let Some(google_client) = loaded_config_after_local_toml.oauth_clients.get("google") {
            assert_eq!(google_client.client_id.to_string(), "YOUR GOOGLE CLIENT ID");
            assert_eq!(
                google_client.client_secret.secret(),
                "YOUR GOOGLE CLIENT SECRET"
            );
        } else {
            panic!("Google client configuration not found");
        }

        if let Some(github_client) = loaded_config_after_local_toml.oauth_clients.get("github") {
            assert_eq!(github_client.client_id.to_string(), "xxx");
            assert_eq!(github_client.client_secret.secret(), "xxx");
        } else {
            panic!("GitHub client configuration not found");
        }
    }
}
