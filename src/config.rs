use std::str::FromStr;

use dotenvy::var;

use crate::errors::app_error::AppError;

#[derive(Clone)]
pub struct Config {
    pub environment: Environment,
    pub port: u16,
    pub database_url: String,
    pub database_max_connections: u32,
    pub redis_url: String,
    pub rate_limit_per_second: u64,
    pub rate_limit_burst: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Environment {
    Local,
    Staging,
    Production,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        let environment = match required_env("ENVIRONMENT")?.as_str() {
            "local" => Environment::Local,
            "staging" => Environment::Staging,
            "production" => Environment::Production,
            other => {
                return Err(AppError::InvalidConfig {
                    variable: String::from("ENVIRONMENT"),
                    value: other.to_string(),
                });
            }
        };

        let port = parse_env("PORT", 3000u16)?;

        let database_max_connections = parse_env("DB_MAX_CONNECTION", 5u32)?;
        if database_max_connections == 0 {
            return Err(AppError::InvalidConfig {
                variable: String::from("DB_MAX_CONNECTION"),
                value: String::from("0"),
            });
        }

        let rate_limit_per_second = parse_env("RATE_LIMIT_PER_SECOND", 10u64)?;
        if rate_limit_per_second == 0 {
            return Err(AppError::InvalidConfig {
                variable: String::from("RATE_LIMIT_PER_SECOND"),
                value: String::from("0"),
            });
        }

        let rate_limit_burst = parse_env("RATE_LIMIT_BURST", 30u32)?;
        if rate_limit_burst == 0 {
            return Err(AppError::InvalidConfig {
                variable: String::from("RATE_LIMIT_BURST"),
                value: String::from("0"),
            });
        }

        // On `local` both URLs come from the mock containers in
        // `utils/mocked_db_connection.rs`, so whatever is set here is unused.
        let (database_url, redis_url) = if environment == Environment::Local {
            (String::new(), String::new())
        } else {
            (required_env("DATABASE_URL")?, required_env("REDIS_URL")?)
        };

        Ok(Config {
            environment,
            port,
            database_url,
            database_max_connections,
            redis_url,
            rate_limit_per_second,
            rate_limit_burst,
        })
    }
}

fn required_env(variable: &str) -> Result<String, AppError> {
    var(variable)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::MissingConfig(String::from(variable)))
}

fn parse_env<T: FromStr>(variable: &str, default: T) -> Result<T, AppError> {
    match var(variable) {
        Ok(value) if !value.trim().is_empty() => {
            value
                .trim()
                .parse::<T>()
                .map_err(|_| AppError::InvalidConfig {
                    variable: String::from(variable),
                    value,
                })
        }
        _ => Ok(default),
    }
}
