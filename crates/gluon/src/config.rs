//! Application settings loaded from `gluon.toml`.

use std::io;
use std::path::Path;
use std::sync::OnceLock;

use serde::Deserialize;

static EMBEDDED_CONFIG: OnceLock<&'static str> = OnceLock::new();

/// Called by the generated application router to provide its build-time config.
#[doc(hidden)]
pub fn __register_embedded_config(source: &'static str) {
    EMBEDDED_CONFIG.get_or_init(|| source);
}

/// Application configuration loaded from `gluon.toml`.
/// `Config::load` falls back to its build-time copy when needed.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub app: AppConfig,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub view: ViewConfig,
    pub telemetry: TelemetryConfig,
}

impl Config {
    /// Loads `gluon.toml` from the current directory, falling back to the build-time copy.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or contains invalid configuration.
    pub fn load() -> io::Result<Self> {
        Self::load_with_embedded("gluon.toml", EMBEDDED_CONFIG.get().copied())
    }

    /// Loads configuration from `path`, falling back to defaults if it does not exist.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or contains invalid configuration.
    pub fn load_from(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::load_with_embedded(path, None)
    }

    fn load_with_embedded(path: impl AsRef<Path>, embedded: Option<&str>) -> io::Result<Self> {
        let path = path.as_ref();
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return match embedded {
                    Some(source) => Self::parse(source, "embedded gluon.toml"),
                    None => Ok(Self::default()),
                };
            }
            Err(error) => {
                return Err(io::Error::new(
                    error.kind(),
                    format!("failed to read {}: {error}", path.display()),
                ));
            }
        };
        Self::parse(&source, path.display())
    }

    fn parse(source: &str, source_name: impl std::fmt::Display) -> io::Result<Self> {
        let config: Self = toml::from_str(source).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid {source_name}: {error}"),
            )
        })?;
        if config.database.driver != "postgres" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid {source_name}: database.driver must be \"postgres\"",),
            ));
        }
        if config.database.pool_size == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid {source_name}: database.pool_size must be greater than zero"),
            ));
        }
        Ok(config)
    }
}

/// Application identity used by the scaffold and available to application code.
#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    pub name: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "gluon".to_owned(),
        }
    }
}

/// HTTP listener settings.
#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_owned(),
            port: 3000,
        }
    }
}

/// PostgreSQL connection settings.
#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DatabaseConfig {
    /// The supported database backend, retained for compatibility with old scaffolds.
    pub driver: String,
    pub url_env: String,
    pub pool_size: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            driver: "postgres".to_owned(),
            url_env: "DATABASE_URL".to_owned(),
            pool_size: 10,
        }
    }
}

/// JSX rendering settings.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ViewConfig {
    pub pretty: bool,
    pub tailwind: bool,
}

/// OpenTelemetry settings.
#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TelemetryConfig {
    pub enabled_env: String,
    pub service_name: String,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled_env: "OTEL_ENABLED".to_owned(),
            service_name: "gluon".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_uses_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::load_from(dir.path().join("gluon.toml")).unwrap();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.database.driver, "postgres");
        assert_eq!(config.database.url_env, "DATABASE_URL");
        assert_eq!(config.database.pool_size, 10);
        assert!(!config.view.pretty);
        assert!(!config.view.tailwind);
        assert_eq!(config.telemetry.enabled_env, "OTEL_ENABLED");
    }

    #[test]
    fn uses_embedded_config_when_runtime_file_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::load_with_embedded(
            dir.path().join("gluon.toml"),
            Some("[app]\nname = \"embedded\"\n[view]\ntailwind = true\n"),
        )
        .unwrap();
        assert_eq!(config.app.name, "embedded");
        assert!(config.view.tailwind);
    }

    #[test]
    fn runtime_config_overrides_embedded_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gluon.toml");
        std::fs::write(
            &path,
            "[app]\nname = \"runtime\"\n[view]\ntailwind = false\n",
        )
        .unwrap();

        let config = Config::load_with_embedded(
            path,
            Some("[app]\nname = \"embedded\"\n[view]\ntailwind = true\n"),
        )
        .unwrap();
        assert_eq!(config.app.name, "runtime");
        assert!(!config.view.tailwind);
    }

    #[test]
    fn reads_scaffolded_settings_and_rejects_unsupported_database_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gluon.toml");
        std::fs::write(
            &path,
            "[app]\nname = \"example\"\n[server]\nhost = \"127.0.0.1\"\nport = 8080\n[database]\ndriver = \"postgres\"\nurl_env = \"APP_DATABASE_URL\"\npool_size = 17\n[view]\npretty = true\ntailwind = true\n[telemetry]\nenabled_env = \"APP_OTEL_ENABLED\"\nservice_name = \"example-service\"\n",
        )
        .unwrap();

        let config = Config::load_from(&path).unwrap();
        assert_eq!(config.app.name, "example");
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.database.driver, "postgres");
        assert_eq!(config.database.url_env, "APP_DATABASE_URL");
        assert_eq!(config.database.pool_size, 17);
        assert!(config.view.pretty);
        assert!(config.view.tailwind);
        assert_eq!(config.telemetry.enabled_env, "APP_OTEL_ENABLED");
        assert_eq!(config.telemetry.service_name, "example-service");

        std::fs::write(&path, "[database]\ndriver = \"mysql\"\n").unwrap();
        let error = Config::load_from(&path).unwrap_err();
        assert!(error.to_string().contains("database.driver"));

        std::fs::write(&path, "[database]\nunsupported = true\n").unwrap();
        let error = Config::load_from(&path).unwrap_err();
        assert!(error.to_string().contains("unsupported"));
    }

    #[test]
    fn rejects_zero_pool_size() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gluon.toml");
        std::fs::write(&path, "[database]\npool_size = 0\n").unwrap();
        let error = Config::load_from(&path).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("pool_size must be greater than zero")
        );
    }
}
