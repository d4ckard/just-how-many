use secrecy::{Secret, ExposeSecret};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};

#[derive(Clone, serde::Deserialize)]
pub struct Settings {
    pub postgres: PostgresSettings,
    pub redis: RedisSettings,
    pub application: ApplicationSettings,
}

#[derive(Clone, serde::Deserialize)]
pub struct PostgresSettings {
    pub username: String,
    pub password: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

impl PostgresSettings {
    pub fn without_db(&self) -> PgConnectOptions {
        let ssl_mode =  if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };

        PgConnectOptions::new()
            .username(&self.username)
            .host(&self.host)
            .port(self.port)
            .password(self.password.expose_secret())
            .ssl_mode(ssl_mode)
    }

    pub fn with_db(&self) -> PgConnectOptions {
        self.without_db().database(&self.database_name)
    }
}

#[derive(Clone, serde::Deserialize)]
pub struct RedisSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub username: Option<String>,
    pub password: Option<Secret<String>>,
}

impl RedisSettings {
    pub fn with_db(&self) -> redis::ConnectionInfo {
	redis::ConnectionInfo {
	    addr: redis::ConnectionAddr::Tcp(self.host.clone(), self.port),
	    redis: redis::RedisConnectionInfo {
		db: 0,
		username: self.username.clone(),
		password: self.password.clone()
		    .map(|p| p.expose_secret().to_owned()),
	    }
	}
    }
}

#[derive(Clone, serde::Deserialize)]
pub struct ApplicationSettings {
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    /// Number of seconds until a visit by the same
    /// IP address counts as a new visit again.
    pub visit_duration: u64,
}

pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String>  for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported enviroment. \
                Use either `local` or `production`.", other
            ))
        }
    }
}


pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir()
        .expect("Failed to determine  the current directory");
    let configuration_directory = base_path.join("configuration");
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT");
    let environment_filename = format!("{}.yaml", environment.as_str());

    let settings = config::Config::builder()
        .add_source(
            config::File::from(configuration_directory.join("base.yaml"))
        )
        .add_source(
            config::File::from(configuration_directory.join(&environment_filename))
        )
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__")
        )
        .build()?;
    settings.try_deserialize::<Settings>()
}
