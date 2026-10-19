//! Core building blocks of Nodeloom: frame splitting, the [`Runner`] trait,
//! resource sampling, device identity and token checks. Networking lives in
//! the agent and master crates.

pub mod auth;
pub mod discovery;
pub mod identity;
pub mod metrics;
pub mod private_file;
pub mod runner;
pub mod split;
pub mod validate;

pub use runner::{FakeRunner, Runner, RunnerError, RunnerSet};
