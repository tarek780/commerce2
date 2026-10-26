use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_address: String,
    pub database_url: String,
    pub redis_url: String,

    pub secret_key: String,
    pub jwt_access_expiry_secs: i64,
    pub jwt_refresh_expiry_secs: i64,
    pub jwt_issuer: String,
    pub jwt_audience: String,

    pub argon2_memory_cost: u32,
    pub argon2_time_cost: u32,
    pub argon2_parallelism: u32,

    pub email_verification_expiry_secs: u64,
    pub email_verify_rate_limit_secs: u64,

    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from: String,
    pub app_public_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            bind_address: get("BIND_ADDRESS")?,
            database_url: get("DATABASE_URL")?,
            redis_url: get("REDIS_URL")?,

            secret_key: get_secret("SECRSECRET_KEY")?,
            jwt_access_expiry_secs: parse_duration_secs(&get("JWT_ACCESS_EXPIRY")?)?,
            jwt_refresh_expiry_secs: parse_duration_secs(&get("JWT_REFRESH_EXPIRY")?)?,
            jwt_issuer: get("JWT_ISSUER")?,
            jwt_audience: get("JWT_AUDIENCE")?,

            argon2_memory_cost: get_parse("ARGON2_MEMORY_COST")?,
            argon2_time_cost: get_parse("ARGON2_TIME_COST")?,
            argon2_parallelism: get_parse("ARGON2_PARALLELISM")?,

            email_verification_expiry_secs: get_parse("EMAIL_VERIFICATION_EXPIRY")?,
            email_verify_rate_limit_secs: get_parse("EMAIL_VERIFY_RATE_LIMIT_SECS")?,

            smtp_host: get("SMTP_HOST")?,
            smtp_port: get_parse("SMTP_PORT")?,
            smtp_username: get("SMTP_USERNAME")?,
            smtp_password: get("SMTP_PASSWORD")?,
            smtp_from: get("SMTP_FROM")?,
            app_public_url: get("APP_PUBLIC_URL")?,
        })
    }
}

// ===== أخطاء الإعداد =====

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing env var: {0}")]
    Missing(String),
    #[error("invalid value for {key}: {source}")]
    Invalid {
        key: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("invalid duration for {key}: {value}")]
    BadDuration { key: String, value: String },
    #[error("{0} must be at least {1} bytes long (got {2})")]
    TooShort(String, usize, usize),
}

// ===== Helpers =====

fn get(key: &str) -> Result<String, ConfigError> {
    env::var(key).map_err(|_| ConfigError::Missing(key.into()))
}

fn get_secret(key: &str) -> Result<String, ConfigError> {
    let v = get(key)?;
    if v.len() < 32 {
        return Err(ConfigError::TooShort(key.into(), 32, v.len()));
    }
    Ok(v)
}

fn get_parse<T>(key: &str) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    let raw = get(key)?;
    raw.parse::<T>().map_err(|e| ConfigError::Invalid {
        key: key.into(),
        source: Box::new(e),
    })
}

fn parse_duration_secs(s: &str) -> Result<i64, ConfigError> {
    let key = "duration".to_string();
    let s = s.trim();

    let (num, mult) = if let Some(v) = s.strip_suffix('s') {
        (v, 1i64)
    } else if let Some(v) = s.strip_suffix('m') {
        (v, 60)
    } else if let Some(v) = s.strip_suffix('h') {
        (v, 3600)
    } else if let Some(v) = s.strip_suffix('d') {
        (v, 86400)
    } else {
        (s, 1)
    };

    let n: i64 = num.trim().parse().map_err(|_| ConfigError::BadDuration {
        key: key.clone(),
        value: s.into(),
    })?;

    Ok(n * mult)
}
