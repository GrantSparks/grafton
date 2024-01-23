#![allow(clippy::module_name_repetitions)]

use grafton_config::{GraftonConfig, GraftonConfigProvider};

use std::{collections::HashMap, net::IpAddr};

use crate::Error;

use {
    derivative::Derivative,
    oauth2::{ClientId, ClientSecret},
    serde::{Deserialize, Serialize},
    serde_json::{Map, Value},
    strum::{Display, EnumString, EnumVariantNames},
    url::Url,
};

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
    Default, EnumString, EnumVariantNames, Debug, Serialize, Deserialize, Clone, PartialEq, Eq,
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
    #[derivative(Default(value = "String::new()"))]
    pub public_home: String,
    #[derivative(Default(value = "\"error\".into()"))]
    pub public_error: String,
    #[derivative(Default(value = "\"login\".into()"))]
    pub public_login: String,
    #[derivative(Default(value = "\"protected\".into()"))]
    pub protected_home: String,
}

impl Pages {
    /// Returns a new `Pages` struct with the `root` path prepended to all paths.
    pub fn with_root(&self) -> Self {
        let normalized_base = normalize_slash(&self.root);
        Self {
            root: normalized_base.clone(),
            public_home: join_paths(&normalized_base, &self.public_home),
            public_error: join_paths(&normalized_base, &self.public_error),
            public_login: join_paths(&normalized_base, &self.public_login),
            protected_home: join_paths(&normalized_base, &self.protected_home),
        }
    }
}

fn normalize_slash(path: &str) -> String {
    if path.ends_with('/') {
        path.to_string()
    } else {
        format!("{path}/")
    }
}

fn join_paths(base: &str, path: &str) -> String {
    let trimmed_base = base.trim_end_matches('/');
    let trimmed_path = path.trim_start_matches('/');
    format!("{trimmed_base}/{trimmed_path}")
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
                eprintln!("Error generating URL: {err}");
                String::new()
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

    #[allow(clippy::missing_const_for_fn)]
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

    fn format_url(&self, protocol: &str, port: u16) -> Result<String, Error> {
        let base = format!("{}://{}", protocol, self.public_hostname);
        let mut url = Url::parse(&base).map_err(|e| Error::UrlFormatError {
            protocol: protocol.to_string(),
            hostname: self.public_hostname.clone(),
            port,
            inner: e,
            cause: "Invalid URL".to_string(),
        })?;

        if !Self::is_default_port(protocol, port) {
            url.set_port(Some(port))
                .map_err(|()| Error::UrlFormatError {
                    protocol: protocol.to_string(),
                    hostname: self.public_hostname.clone(),
                    port,
                    cause: "Invalid port".to_string(),
                    inner: url::ParseError::InvalidPort,
                })?;
        }

        Ok(url.to_string().trim_end_matches('/').to_string())
    }
}

#[derive(
    Default,
    Display,
    EnumString,
    EnumVariantNames,
    Debug,
    Serialize,
    Deserialize,
    Clone,
    PartialEq,
    Eq,
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
    pub display_name: String,
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
    pub auth_uri: String,
    pub token_uri: String,
    #[serde(default)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Derivative, Clone)]
#[derivative(Default)]
#[serde(default)]
pub struct Config {
    #[serde(flatten)]
    pub base: GraftonConfig,
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

impl GraftonConfigProvider for Config {
    fn get_grafton_config(&self) -> &GraftonConfig {
        &self.base
    }
}

#[cfg(test)]
mod tests {
    use grafton_config::{load_config_from_dir, GraftonConfigProvider};

    use crate::ServerConfigProvider;

    use super::*;

    #[test]
    fn test_base_prepend() {
        let pages = Pages {
            root: "/api".to_string(),
            public_home: "home".to_string(),
            public_error: "/error".to_string(),
            public_login: "login".to_string(),
            protected_home: "protected".to_string(),
        };

        let updated_pages = pages.with_root();
        assert_eq!(updated_pages.public_home, "/api/home");
        assert_eq!(updated_pages.public_error, "/api/error");
        assert_eq!(updated_pages.public_login, "/api/login");
        assert_eq!(updated_pages.protected_home, "/api/protected");
    }

    #[test]
    fn test_base_prepend_with_trailing_slash() {
        let pages = Pages {
            root: "/api/".to_string(),
            public_home: "home".to_string(),
            public_error: "error".to_string(),
            public_login: "login".to_string(),
            protected_home: "protected".to_string(),
        };

        let updated_pages = pages.with_root();
        assert_eq!(updated_pages.public_home, "/api/home");
        assert_eq!(updated_pages.public_error, "/api/error");
        assert_eq!(updated_pages.public_login, "/api/login");
        assert_eq!(updated_pages.protected_home, "/api/protected");
    }

    #[test]
    fn test_base_prepend_with_empty_and_root_pages() {
        let pages = Pages {
            root: "/".to_string(),
            public_home: String::new(),
            public_error: "/".to_string(),
            public_login: "/".to_string(),
            protected_home: "/".to_string(),
        };

        let updated_pages = pages.with_root();
        assert_eq!(updated_pages.public_home, "/");
        assert_eq!(updated_pages.public_error, "/");
        assert_eq!(updated_pages.public_login, "/");
        assert_eq!(updated_pages.protected_home, "/");
    }

    #[allow(clippy::similar_names)]
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
        assert_eq!(normalize_slash("path"), "path/");
        assert_eq!(normalize_slash("path/"), "path/");
    }

    #[test]
    fn test_join_paths() {
        assert_eq!(join_paths("/root", "/path"), "/root/path");
        assert_eq!(join_paths("/root/", "/path"), "/root/path");
        assert_eq!(join_paths("/root", "path"), "/root/path");
        assert_eq!(join_paths("/root/", "path"), "/root/path");
        assert_eq!(join_paths("/root/", "//path"), "/root/path");
    }

    #[test]
    fn test_with_base_prepend() {
        let pages = Pages {
            root: "/api".to_string(),
            public_home: "home".to_string(),
            public_error: "/error".to_string(),
            public_login: "login".to_string(),
            protected_home: "protected".to_string(),
        };

        let new_pages = pages.with_root();
        assert_eq!(new_pages.root, "/api/");
        assert_eq!(new_pages.public_home, "/api/home");
        assert_eq!(new_pages.public_error, "/api/error");
        assert_eq!(new_pages.public_login, "/api/login");
        assert_eq!(new_pages.protected_home, "/api/protected");
    }

    #[test]
    fn test_load_single_oauth_client() {
        let json = r#"
        {
            "run_mode": "test",
            "oauth_clients": {
                "github": {
                    "display_name": "GitHub",
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
                    "display_name": "GitHub",
                    "client_id": "github_id",
                    "client_secret": "github_secret",
                    "redirect_uri": "http://localhost/github/redirect",
                    "auth_uri": "http://localhost/github/callback",
                    "token_uri": "http://localhost/github/token",
                    "extra": { 
                        "userinfo_uri": "https://api.github.com/user"
                    }
                },
                "google": {
                    "display_name": "Google",
                    "client_id": "google_id",
                    "client_secret": "google_secret",
                    "redirect_uri": "http://localhost/google/redirect",
                    "auth_uri": "http://localhost/google/callback",
                    "token_uri": "http://localhost/google/token",
                    "extra": { 
                        "userinfo_uri": "https://www.googleapis.com/oauth2/v3/userinfo" 
                    }
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

        let userinfo_uri = config
            .oauth_clients
            .get("google")
            .unwrap()
            .extra
            .get("userinfo_uri")
            .unwrap();
        assert_eq!(
            userinfo_uri,
            "https://www.googleapis.com/oauth2/v3/userinfo"
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

        let result: Result<GraftonConfig, _> = serde_json::from_str(json);
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

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct TestConfig {
        #[serde(flatten)]
        pub base: Config,
    }

    impl GraftonConfigProvider for TestConfig {
        fn get_grafton_config(&self) -> &GraftonConfig {
            self.base.get_grafton_config()
        }
    }

    impl ServerConfigProvider for TestConfig {
        fn get_server_config(&self) -> &Config {
            &self.base
        }
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
            google = { display_name = "Google", client_id = "YOUR GOOGLE CLIENT ID", client_secret = "YOUR GOOGLE CLIENT SECRET", auth_uri = "https://accounts.google.com/o/oauth2/auth", token_uri = "https://oauth2.googleapis.com/token" }
            github = { display_name = "GitHub", client_id = "xxx", client_secret = "xxx", auth_uri = "https://github.com/login/oauth/authorize", token_uri = "https://github.com/login/oauth/access_token" }
        "#;

        let local_toml_content = r#"
            [logger]
            verbosity = "Debug"
    
            [website]
            bind_ssl_config = { enabled = false }
            public_ports = { http = 80, https = 443 }
            public_ssl_enabled = true
        "#;

        let temp_dir = tempfile::tempdir().expect("Failed to create a temporary directory");
        let config_dir = temp_dir.path();

        let default_path = config_dir.join("default.toml");
        let local_path = config_dir.join("local.toml");

        std::fs::write(default_path, config_toml_content)
            .expect("Failed to write to temp default.toml file");
        std::fs::write(local_path, local_toml_content)
            .expect("Failed to write to temp local.toml file");
        let loaded_config_after_local_toml: TestConfig =
            load_config_from_dir(config_dir.to_str().unwrap())
                .expect("Failed to load config after local.toml");

        assert_eq!(
            loaded_config_after_local_toml.get_grafton_config().run_mode,
            "dev"
        );
        assert_eq!(
            loaded_config_after_local_toml
                .get_server_config()
                .website
                .bind_ports
                .http,
            8080
        );
        assert_eq!(
            loaded_config_after_local_toml
                .get_server_config()
                .website
                .public_ports
                .http,
            80
        );
        assert_eq!(
            loaded_config_after_local_toml
                .get_server_config()
                .website
                .public_ports
                .https,
            443
        );
        assert!(
            loaded_config_after_local_toml
                .get_server_config()
                .website
                .public_ssl_enabled
        );
        assert_eq!(
            loaded_config_after_local_toml
                .get_server_config()
                .logger
                .verbosity,
            Verbosity::Debug
        );

        loaded_config_after_local_toml
            .get_server_config()
            .oauth_clients
            .get("google")
            .map_or_else(
                || {
                    panic!("Google client configuration not found");
                },
                |google_client| {
                    assert_eq!(google_client.client_id.to_string(), "YOUR GOOGLE CLIENT ID");
                    assert_eq!(
                        google_client.client_secret.secret(),
                        "YOUR GOOGLE CLIENT SECRET"
                    );
                },
            );

        loaded_config_after_local_toml
            .get_server_config()
            .oauth_clients
            .get("github")
            .map_or_else(
                || {
                    panic!("GitHub client configuration not found");
                },
                |github_client| {
                    assert_eq!(github_client.client_id.to_string(), "xxx");
                    assert_eq!(github_client.client_secret.secret(), "xxx");
                },
            );
    }
}
