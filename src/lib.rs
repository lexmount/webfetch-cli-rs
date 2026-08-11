//! Native Lexmount WebFetch SDK and agent-friendly output helpers.

pub mod auth;
pub mod client;
pub mod error;
pub mod output;

pub use client::{Client, ClientBuilder};
pub use error::{Error, Result};
