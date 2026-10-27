pub mod config;
pub mod db;
pub mod error;
pub mod graphql;
pub mod grpc;
pub mod helpers;
pub mod outbox;
pub mod redis;
pub mod state;
pub mod stream_worker;
pub mod middleware;

pub mod pb_inv {
    tonic::include_proto!("inventory.v1");
}