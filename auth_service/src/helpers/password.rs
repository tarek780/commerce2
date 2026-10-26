use argon2::{Config as Argon2Config, Variant, Version};
use crate::error::AppError;

pub fn hash_password(password: &str, memory_cost: u32, time_cost: u32, parallelism: u32) -> Result<String, AppError> {
    let cfg = Argon2Config {
        variant: Variant::Argon2id,
        version: Version::Version13,
        mem_cost: memory_cost,
        time_cost,
        lanes: parallelism,
        secret: &[],
        ad: &[],
        hash_length: 32,
    };
    let salt = rand::random::<[u8; 16]>();
    let hash = argon2::hash_encoded(password.as_bytes(), &salt, &cfg)
        .map_err(|e| AppError::Internal(format!("argon2 hash: {e}")))?;
    Ok(hash)
}

pub fn verify_password(password: &str, encoded: &str) -> Result<bool, AppError> {
    argon2::verify_encoded(encoded, password.as_bytes())
        .map_err(|e| AppError::Internal(format!("argon2 verify: {e}")))
}