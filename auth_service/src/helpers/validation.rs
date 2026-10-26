use validator::Validate;
use crate::error::AppError;

pub fn validate<T: Validate>(v: &T) -> Result<(), AppError> {
    v.validate()
        .map_err(|e| AppError::Validation(e.to_string()))
}

pub fn require_verified_email(verified: bool) -> Result<(), AppError> {
    if !verified {
        return Err(AppError::Forbidden);
    }
    Ok(())
}