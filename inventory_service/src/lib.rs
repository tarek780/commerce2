pub mod error;
pub mod inventory;
pub mod outbox;

pub mod pb {
    tonic::include_proto!("inventory.v1");
}
