//! Core building blocks of Nodeloom: frame splitting, the [`Runner`] trait,
//! resource sampling and token checks. Networking lives in the agent and
//! master crates.

pub mod auth;
pub mod discovery;
pub mod metrics;
pub mod runner;
pub mod split;
pub mod validate;

pub use runner::{FakeRunner, Runner, RunnerError, RunnerSet};
