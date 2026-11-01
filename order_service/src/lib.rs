pub mod clients;
pub mod config;
pub mod error;
pub mod kafka;
pub mod order;
pub mod outbox;
pub mod state;

pub mod pb_order { tonic::include_proto!("order.v1"); }
pub mod pb_inv   { tonic::include_proto!("inventory.v1"); }
pub mod pb_pay   { tonic::include_proto!("payment.v1"); }