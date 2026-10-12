//! Core building blocks of Nodeloom: frame splitting, the [`Runner`] trait,
//! resource sampling and token checks. Networking lives in the agent and
//! master crates.

pub mod auth;
pub mod metrics;
pub mod runner;
pub mod split;

pub use runner::{FakeRunner, Runner, RunnerError, RunnerSet};
