#![cfg(any(test, feature = "testing"))]

// Testing utilities and mocks for akd-watch-common
//
// This module provides realistic mock implementations of storage and repository traits
// for testing purposes. These mocks simulate the behavior of real implementations
// without requiring external dependencies like databases or AKD services.
//
// Usage:
// - Use these mocks to test individual components in isolation
// - Configure failure modes to test error handling scenarios
// - Inspect internal state to verify expected behavior
//
// Note: These are designed for unit and component testing with mocked dependencies

pub mod mock_namespace_repository;
pub mod mock_signature_storage;
pub mod mock_signing_key_repository;

pub use mock_namespace_repository::MockNamespaceRepository;
pub use mock_signature_storage::MockSignatureStorage;
pub use mock_signing_key_repository::{MockSigningKeyRepository, MockVerifyingKeyRepository};
