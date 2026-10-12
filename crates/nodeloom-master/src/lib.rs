//! The Nodeloom master: tracks agents, splits jobs across them and serves the
//! cluster state to UI clients.

pub mod server;
pub mod state;

pub use server::{router, serve};
pub use state::{Cluster, SubmitError};

/// A fresh random pairing token (122 bits from the OS random source).
pub fn generate_token() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}
