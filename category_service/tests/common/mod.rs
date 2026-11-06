#![allow(dead_code)]
use std::sync::Arc;

use ::redis::aio::*;
use sqlx::{Pool, Postgres};
use testcontainers::{ContainerAsync, GenericImage};

pub mod db;
pub mod redis;

pub struct TestStatePG {
    pub pool: Arc<Pool<Postgres>>,
    pub pg_container: ContainerAsync<GenericImage>,
}

pub struct TestStateRD {
    pub redis_container: ContainerAsync<GenericImage>,
    pub redis_manager: ConnectionManager,
}
