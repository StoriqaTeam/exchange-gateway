use std::env;

use sentry_integration::SentryConfig;

use config_crate::{Config as RawConfig, ConfigError, Environment, File};

use logger::{FileLogConfig, GrayLogConfig};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: Server,
    pub database: Database,
    pub client: Client,
    pub auth: Auth,
    pub cpu_pool: CpuPool,
    pub exchange_options: Options,
    pub limits: CurrenciesLimits,
    pub sentry: Option<SentryConfig>,
    pub graylog: Option<GrayLogConfig>,
    pub filelog: Option<FileLogConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Client {
    pub dns_threads: usize,
    pub exmo_url: String,
    pub timeout_s: Option<u64>,
    pub retry_attempts: Option<usize>,
    pub retry_timeout: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Auth {
    pub exmo_api_key: String,
    pub exmo_api_secret: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Options {
    pub expiration: u64, // seconds
    pub rate_upside: f64,
    pub safety_threshold: f64,
    pub test_environment: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub host: String,
    pub port: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Database {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CpuPool {
    pub size: usize,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct CurrenciesLimits {
    pub stq: Limits,
    pub btc: Limits,
    pub eth: Limits,
    pub usd: Limits,
    pub rub: Limits,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Limits {
    pub min: f64,
    pub max: f64,
}

impl Default for Limits {
    fn default() -> Self {
        Self { min: 0f64, max: 1f64 }
    }
}

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = RawConfig::new();
        s.merge(File::with_name("config/base"))?;

        // Merge development.toml if RUN_MODE variable is not set
        let env = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
        s.merge(File::with_name(&format!("config/{}", env)).required(false))?;
        s.merge(File::with_name("config/secret.toml").required(false))?;

        s.merge(Environment::with_prefix("STQ_PAYMENTS"))?;
        s.try_into()
    }
}
