/*
 * Copyright FalkorDB Ltd. 2023 - present
 * Licensed under the Server Side Public License v1 (SSPLv1).
 */

#[cfg(not(feature = "redis"))]
compile_error!("The `redis` feature must be enabled.");

mod client;
mod connection;
mod connection_info;
mod error;
mod graph;
mod graph_schema;
mod parser;
mod response;
mod value;

#[cfg(feature = "redis")]
mod redis_ext;

pub use client::{blocking::FalkorSyncClient, builder::FalkorClientBuilder};
pub use connection_info::FalkorConnectionInfo;
pub use error::FalkorDBError;
pub use graph::blocking::SyncGraph;
pub use graph_schema::{blocking::SyncGraphSchema, SchemaType};
pub use parser::FalkorParsable;
pub use response::{
    constraint::{Constraint, ConstraintStatus, ConstraintType},
    execution_plan::ExecutionPlan,
    query_result::QueryResult,
    slowlog_entry::SlowlogEntry,
    ResponseVariant,
};
pub use value::{
    config::ConfigValue,
    graph_entities::{Edge, EntityType, Node},
    path::Path,
    point::Point,
    FalkorValue,
};

#[cfg(feature = "tokio")]
pub use {
    client::asynchronous::FalkorAsyncClient, connection::asynchronous::FalkorAsyncConnection,
    graph::asynchronous::AsyncGraph, graph_schema::asynchronous::AsyncGraphSchema,
    parser::FalkorAsyncParseable,
};
