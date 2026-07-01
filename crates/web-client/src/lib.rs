//! Verifying client for the `akd_watch_web` HTTP API.
//!
//! The [`Client`] is bound to a single `(base_url, namespace)`. It fetches
//! audit signatures from the akd-watch and verifies them against either a
//! caller-supplied set of pinned [`VerifyingKey`]s or — when no keys are
//! pinned — whatever keys the server reports on its `/info` endpoint, with
//! no prior verification of those keys.
//!
//! Each public method returns a method-specific error type whose variants are
//! exactly those reachable from that call: [`BuildError`], [`HttpError`],
//! [`VerifyAuditError`]. Once `build` succeeds, the namespace is guaranteed
//! to exist — `verify_audit` and `namespace_info` cannot return a
//! "namespace not found" error.
//!
//! # Example
//!
//! ```no_run
//! use akd_watch_web_client::{AuditOptions, Client, BuildError};
//! use akd_watch_protocol::Epoch;
//!
//! # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::builder("https://akd-watch.example.com", "namespace-a")
//!     .build()
//!     .await?;
//!
//! let root_hash = client
//!     .verify_audit(Epoch::new(42), AuditOptions::default())
//!     .await?;
//! # let _ = root_hash;
//! # Ok(())
//! # }
//! ```

mod client;
mod error;
mod keys;
mod verify;
mod wire;

pub use akd_watch_protocol::{
    Ciphersuite, Epoch, NamespaceInfo, NamespaceStatus, crypto::VerifyingKey,
};
pub use client::{AuditOptions, Client, ClientBuilder};
pub use error::{BuildError, HttpError, VerifyAuditError};
