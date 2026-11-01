pub mod config;
pub mod error;
pub mod gateway;
pub mod outbox;
pub mod payment;
pub mod state;

pub mod pb {
    tonic::include_proto!("payment.v1");
}
