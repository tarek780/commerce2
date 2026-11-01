use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, SinglePart},
    transport::smtp::authentication::Credentials,
};
use std::str::FromStr;

use crate::{error::AppError, state::AppState};

pub async fn send_verification_email(
    state: &AppState,
    to: &str,
    token: &str,
) -> Result<(), AppError> {
    let verify_url = format!(
        "{}/api/v1/auth/verify-email?token={}",
        state.config.app_public_url, token
    );

    let from = Mailbox::from_str(&state.config.smtp_from)
        .map_err(|e| AppError::Internal(format!("bad from: {e}")))?;
    let to_mb = Mailbox::from_str(to).map_err(|e| AppError::Internal(format!("bad to: {e}")))?;

    let body_part = SinglePart::plain(format!("Click to verify: {verify_url}"));

    let email = Message::builder()
        .from(from)
        .to(to_mb)
        .subject("Verify your email")
        .singlepart(body_part) 
        .map_err(|e| AppError::Internal(format!("build email: {e}")))?;

    tracing::info!(to = %to, "sending verification email");

    let creds = Credentials::new(
        state.config.smtp_username.clone(),
        state.config.smtp_password.clone(),
    );

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&state.config.smtp_host)
        .map_err(|e| AppError::Internal(format!("smtp relay init failed: {e}")))?
        .credentials(creds)
        .build();

    mailer
        .send(email)
        .await
        .map_err(|e| AppError::Internal(format!("smtp send failed: {e}")))?;

    Ok(())
}
