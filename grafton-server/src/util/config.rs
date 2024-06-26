#![allow(clippy::module_name_repetitions)]

use std::{collections::HashMap, net::IpAddr, str::FromStr as _};

use grafton_config::{GraftonConfig, GraftonConfigProvider, TokenExpandingConfig};

use crate::Error;

use {
    derivative::Derivative,
    oauth2::{ClientId, ClientSecret},
    oxide_auth::primitives::{
        prelude::*,
        registrar::{ClientMap, IgnoreLocalPortUrl, RegisteredUrl},
    },
    serde::{Deserialize, Serialize},
    serde_json::{Map, Value},
    strum::{Display, EnumString, VariantNames},
    url::Url,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthClientConfig {
    pub allowed_redirect_uris: Vec<String>,
    pub client_secret: ClientSecret,
}

impl Default for AuthClientConfig {
    fn default() -> Self {
        Self {
            allowed_redirect_uris: Vec::new(),
            client_secret: ClientSecret::new(String::new()),
        }
    }
}

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct AuthServerConfig {
    /// Url our clients must post to in order to get an access token.
    pub token_url: String,

    /// Url our clients must post to in order to refresh the access token.
    pub refresh_url: String,

    /// The url our clients must post to in order to obtain an authorization code.
    pub authorize_url: String,

    /// The registered clients who are allowed to authenticate using oauth
    /// The key is the `client_id`
    pub clients: HashMap<String, AuthClientConfig>,
}

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
    Default, EnumString, VariantNames, Debug, Serialize, Deserialize, Clone, PartialEq, Eq,
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
pub struct Routes {
    #[derivative(Default(value = "\"/\".into()"))]
    pub root: String,
    #[derivative(Default(value = "String::new()"))]
    pub public_home: String,
    #[derivative(Default(value = "\"error\".into()"))]
    pub public_error: String,
    #[derivative(Default(value = "\"login\".into()"))]
    pub public_login: String,
    #[derivative(Default(value = "\"logout\".into()"))]
    pub public_logout: String,
    #[derivative(Default(value = "\"protected\".into()"))]
    pub protected_home: String,
}

impl Routes {
    /// Returns a new `Routes` struct with the `root` path prepended to all paths.
    pub fn with_root(&self) -> Self {
        let normalized_base = normalize_slash(&self.root);
        Self {
            root: normalized_base.clone(),
            public_home: join_paths(&normalized_base, &self.public_home),
            public_error: join_paths(&normalized_base, &self.public_error),
            public_login: join_paths(&normalized_base, &self.public_login),
            public_logout: join_paths(&normalized_base, &self.public_logout),
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
    pub bind_ports: Port,

    #[derivative(Default(value = "\"localhost\".into()"))]
    pub public_hostname: String,

    #[derivative(Default)]
    pub public_ports: Port,

    #[derivative(Default(value = "false"))]
    pub public_ssl_enabled: bool,

    #[derivative(Default)]
    #[serde(default)]
    pub routes: Routes,
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

    const fn get_protocol_and_port(&self) -> (&str, u16) {
        if self.public_ssl_enabled {
            ("https", self.public_ports.https)
        } else {
            ("http", self.public_ports.http)
        }
    }

    pub fn is_default_port(protocol: &str, port: u16) -> bool {
        let defaults = Port::default();
        match protocol {
            "http" => port == defaults.http,
            "https" => port == defaults.https,
            "grpc" => port == defaults.grpc,
            _ => false,
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
    Default, Display, EnumString, VariantNames, Debug, Serialize, Deserialize, Clone, PartialEq, Eq,
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
pub struct Port {
    #[derivative(Default(value = "80"))]
    pub http: u16,
    #[derivative(Default(value = "443"))]
    pub https: u16,
    #[derivative(Default(value = "9339"))]
    pub grpc: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthProviderConfig {
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
    pub oauth_providers: HashMap<String, AuthProviderConfig>,
    #[derivative(Default(value = "Vec::new()"))]
    pub oso_policy_files: Vec<String>,
    pub oauth_server: AuthServerConfig,
}

impl GraftonConfigProvider for Config {
    fn get_grafton_config(&self) -> &GraftonConfig {
        &self.base
    }
}

impl TokenExpandingConfig for Config {}

impl TryInto<ClientMap> for Config {
    type Error = Error;

    fn try_into(self) -> Result<ClientMap, Self::Error> {
        let auth_server_config = &self.oauth_server;

        let mut client_map = ClientMap::new();

        for (client_id, client_config) in &auth_server_config.clients {
            let redirect_uris: Result<Vec<RegisteredUrl>, Error> = client_config
                .allowed_redirect_uris
                .iter()
                .map(|uri| {
                    if uri.contains("localhost") {
                        IgnoreLocalPortUrl::from_str(uri)
                            .map(RegisteredUrl::IgnorePortOnLocalhost)
                            .map_err(Error::from)
                    } else {
                        Url::parse(uri)
                            .map(RegisteredUrl::Semantic)
                            .map_err(Error::from)
                    }
                })
                .collect();

            let redirect_uris = redirect_uris?;

            if let Some((first_uri, others)) = redirect_uris.split_first() {
                let client = Client::confidential(
                    client_id,
                    first_uri.clone(),
                    "default".parse::<Scope>().map_err(Error::from)?,
                    client_config.client_secret.secret().as_bytes(),
                )
                .with_additional_redirect_uris(others.to_vec());

                client_map.register_client(client);
            }
        }

        Ok(client_map)
    }
}

#[cfg(test)]
mod tests {
    use grafton_config::{load_config_from_dir, GraftonConfigProvider};

    use crate::ServerConfigProvider;

    use super::*;

    #[test]
    fn test_base_prepend_with_and_without_trailing_slash() {
        let routes_with_slash = Routes {
            root: "/api/".to_string(),
            public_home: "home".to_string(),
            public_error: "error".to_string(),
            public_login: "login".to_string(),
            public_logout: "logout".to_string(),
            protected_home: "protected".to_string(),
        };

        let updated_routes_with_slash = routes_with_slash.with_root();
        assert_eq!(updated_routes_with_slash.public_home, "/api/home");
        assert_eq!(updated_routes_with_slash.public_error, "/api/error");
        assert_eq!(updated_routes_with_slash.public_login, "/api/login");
        assert_eq!(updated_routes_with_slash.public_logout, "/api/logout");
        assert_eq!(updated_routes_with_slash.protected_home, "/api/protected");

        let routes_without_slash = Routes {
            root: "/api".to_string(),
            public_home: "home".to_string(),
            public_error: "error".to_string(),
            public_login: "login".to_string(),
            public_logout: "logout".to_string(),
            protected_home: "protected".to_string(),
        };

        let updated_routes_without_slash = routes_without_slash.with_root();
        assert_eq!(updated_routes_without_slash.public_home, "/api/home");
        assert_eq!(updated_routes_without_slash.public_error, "/api/error");
        assert_eq!(updated_routes_without_slash.public_login, "/api/login");
        assert_eq!(updated_routes_without_slash.public_logout, "/api/logout");
        assert_eq!(
            updated_routes_without_slash.protected_home,
            "/api/protected"
        );
    }

    #[test]
    fn test_base_prepend_with_special_cases() {
        let routes = Routes {
            root: "/".to_string(),
            public_home: String::new(),
            public_error: "/".to_string(),
            public_login: "/".to_string(),
            public_logout: "/".to_string(),
            protected_home: "/".to_string(),
        };

        let updated_routes = routes.with_root();
        assert_eq!(updated_routes.public_home, "/");
        assert_eq!(updated_routes.public_error, "/");
        assert_eq!(updated_routes.public_login, "/");
        assert_eq!(updated_routes.public_logout, "/");
        assert_eq!(updated_routes.protected_home, "/");
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
            public_ports: Port {
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
            "root": "/api",
            "public_home": "/home",
            "public_error": "/error"
        }"#;

        let deserialized: Routes = serde_json::from_str(json).expect("Deserialization failed");
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
        let routes = Routes {
            root: "/api".to_string(),
            public_home: "home".to_string(),
            public_error: "/error".to_string(),
            public_login: "login".to_string(),
            public_logout: "logout".to_string(),
            protected_home: "protected".to_string(),
        };

        let new_routes = routes.with_root();
        assert_eq!(new_routes.root, "/api/");
        assert_eq!(new_routes.public_home, "/api/home");
        assert_eq!(new_routes.public_error, "/api/error");
        assert_eq!(new_routes.public_login, "/api/login");
        assert_eq!(new_routes.public_logout, "/api/logout");
        assert_eq!(new_routes.protected_home, "/api/protected");
    }

    #[test]
    fn test_load_single_oauth_client() {
        let json = r#"
        {
            "run_mode": "test",
            "oauth_providers": {
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
            config.oauth_providers.get("github").unwrap().client_id,
            ClientId::new("github_id".to_string())
        );
        assert_eq!(
            config.oauth_providers.get("github").unwrap().auth_uri,
            "http://localhost/callback"
        );
        assert_eq!(
            config.oauth_providers.get("github").unwrap().token_uri,
            "http://localhost/token"
        );
    }

    #[test]
    fn test_load_multiple_oauth_providers() {
        let json = r#"
        {
            "run_mode": "test",
            "oauth_providers": {
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
            config.oauth_providers.get("github").unwrap().token_uri,
            "http://localhost/github/token"
        );
        assert_eq!(
            config.oauth_providers.get("google").unwrap().token_uri,
            "http://localhost/google/token"
        );

        let userinfo_uri = config
            .oauth_providers
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
            "oauth_providers": {
                "invalid_client": {
                    "client_id": "id_without_secret",
                    // missing fields
                }
            }
        }"#;

        let result: Result<GraftonConfig, _> = serde_json::from_str(json);
        assert!(result.is_err());
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

    impl TokenExpandingConfig for TestConfig {}

    #[test]
    fn test_config_load_with_local_override() {
        let config_toml_content = r#"
            run_mode = "dev"
    
            [website]
            bind_ports = { http = 8080 }
            public_ports = { http = 8080 }
    
            [session]
    
            [routes]
    
            [oauth_providers]
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
            Some("dev".to_string())
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
            .oauth_providers
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
            .oauth_providers
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

    #[test]
    fn test_auth_server_config_custom_initialization() {
        let mut clients = HashMap::new();
        clients.insert(
            "client_identifier_given_to_example.com".to_string(),
            AuthClientConfig {
                allowed_redirect_uris: vec!["http://example.com/callback".to_string()],
                client_secret: ClientSecret::new("secret123".to_string()),
            },
        );

        let config = AuthServerConfig {
            token_url: "http://localhost/token".to_string(),
            refresh_url: "http://localhost/refresh".to_string(),
            authorize_url: "http://localhost/authorize".to_string(),
            clients,
        };

        assert_eq!(config.token_url, "http://localhost/token");
        assert_eq!(config.refresh_url, "http://localhost/refresh");
        assert_eq!(config.authorize_url, "http://localhost/authorize");
        assert!(config
            .clients
            .contains_key("client_identifier_given_to_example.com"));
    }

    #[cfg(test)]
    #[test]
    fn test_is_default_port_http_default() {
        assert!(
            Website::is_default_port("http", 80),
            "HTTP default port should be recognized as default."
        );
    }

    #[test]
    fn test_is_default_port_https_default() {
        assert!(
            Website::is_default_port("https", 443),
            "HTTPS default port should be recognized as default."
        );
    }

    #[test]
    fn test_is_default_port_http_non_default() {
        assert!(
            !Website::is_default_port("http", 8080),
            "HTTP non-default port should not be recognized as default."
        );
    }

    #[test]
    fn test_is_default_port_https_non_default() {
        assert!(
            !Website::is_default_port("https", 8443),
            "HTTPS non-default port should not be recognized as default."
        );
    }

    #[test]
    fn test_is_default_port_unrecognized_protocol() {
        assert!(
            !Website::is_default_port("ftp", 21),
            "Unrecognized protocol should not have a default port."
        );
    }

    #[test]
    fn test_format_url_with_default_http_port() {
        let website = Website {
            public_hostname: "example.com".into(),
            public_ports: Port {
                http: 80,
                https: 443,
                grpc: 9339,
            },
            public_ssl_enabled: false,
            ..Default::default()
        };
        let url = website
            .format_url("http", website.public_ports.http)
            .unwrap();
        assert_eq!(url, "http://example.com");
    }

    #[test]
    fn test_format_url_with_non_default_http_port() {
        let website = Website {
            public_hostname: "example.com".into(),
            public_ports: Port {
                http: 8080,
                https: 443,
                grpc: 9339,
            },
            public_ssl_enabled: false,
            ..Default::default()
        };
        let url = website
            .format_url("http", website.public_ports.http)
            .unwrap();
        assert_eq!(url, "http://example.com:8080");
    }

    #[test]
    fn test_format_url_with_default_https_port() {
        let website = Website {
            public_hostname: "example.com".into(),
            public_ports: Port {
                http: 80,
                https: 443,
                grpc: 9339,
            },
            public_ssl_enabled: true,
            ..Default::default()
        };
        let url = website
            .format_url("https", website.public_ports.https)
            .unwrap();
        assert_eq!(url, "https://example.com");
    }

    #[test]
    fn default_website_config() {
        let default_website = Website::default();
        assert_eq!(default_website.public_hostname, "localhost");
        assert!(!default_website.public_ssl_enabled);
        assert_eq!(default_website.public_ports.http, 80);
    }

    #[test]
    fn default_auth_server_config() {
        let default_auth_server = AuthServerConfig::default();
        assert!(default_auth_server.clients.is_empty());
        assert_eq!(default_auth_server.token_url, "");
    }

    #[test]
    fn test_normalize_slash_variations() {
        assert_eq!(normalize_slash("api"), "api/");
        assert_eq!(normalize_slash("api/"), "api/");
    }

    #[test]
    fn test_join_paths_variations() {
        assert_eq!(join_paths("/api", "endpoint"), "/api/endpoint");
        assert_eq!(join_paths("/api/", "/endpoint"), "/api/endpoint");
    }

    #[test]
    fn test_deserialization_error_for_incomplete_json() {
        let incomplete_json = r#"{
            "token_url": "http://localhost/token"
            // Missing refresh_url and authorize_url
        }"#;

        let result: Result<AuthServerConfig, _> = serde_json::from_str(incomplete_json);
        assert!(result.is_err(), "Should error on incomplete JSON");
    }
}
