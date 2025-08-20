pub mod akd_configurations;
pub mod akd_storage_factory;
mod audit_blob_name;
pub mod config;
pub mod crypto;
mod epoch_signature;
mod error;
mod namespace_info;
pub(crate) mod proto;
pub mod storage;
mod versions;

pub use akd_configurations::BitwardenV1Configuration;
pub use audit_blob_name::SerializableAuditBlobName;
use chrono::Duration;
pub(crate) use epoch_signature::EpochSignedMessage;
pub use epoch_signature::{EpochSignature, SignError, VerifyError};
pub use namespace_info::*;
use tokio::time::Instant;
pub use versions::*;

#[cfg(test)]
pub use akd_configurations::TestAkdConfiguration;

// Export testing utilities when cfg(test) is enabled
#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub const BINCODE_CONFIG: bincode::config::Configuration<
    bincode::config::LittleEndian,
    bincode::config::Varint,
    bincode::config::NoLimit,
> = bincode::config::standard()
    .with_little_endian()
    .with_variable_int_encoding()
    .with_no_limit();

pub async fn tic_toc<T>(f: impl core::future::Future<Output = T>) -> T {
    {
        let tic = Instant::now();
        let out = f.await;
        match Duration::from_std(Instant::now() - tic) {
            Ok(duration) => {
                tracing::debug!("Elapsed time: {:?}", duration);
            }
            Err(e) => {
                tracing::warn!("Failed to calculate elapsed time: {}", e);
            }
        }
        out
    }
}

/// Macro to log timed events with tracing
/// This macro captures the duration of an async operation and logs it at the specified level.
/// It returns a future that must be awaited.
///
/// Usage:
/// ```rust,no_run
/// use akd_watch_common::timed_event;
///
/// async fn example() {
///     // Basic usage with just a future (note the .await)
///     let result = timed_event!(INFO, some_async_function()).await;
///     // With a custom message
///     let result = timed_event!(DEBUG, some_async_function(); "Database query completed").await;
///     // With additional fields
///     let result = timed_event!(INFO, some_async_function(); user_id = 123, table = "users").await;
///     // With fields and message (fields come first)
///     let result = timed_event!(INFO, some_async_function(); user_id = 123, "Query completed").await;
///     
///     // Result-aware logging - access the result value in logging
///     let result = timed_event!(with_result(res) INFO, some_function();
///                               result_len = res.len(), "Operation completed").await;
///     
///     // Result-aware logging with just the result value
///     let status = timed_event!(with_result(code) INFO, get_status_code();
///                               status_code = *code).await;
/// }
///
/// async fn some_async_function() -> i32 { 42 }
/// async fn some_function() -> String { "success".to_string() }
/// async fn get_status_code() -> u16 { 200 }
/// ```
#[macro_export]
macro_rules! timed_event {
    // Basic case: just level and future
    ($level:ident, $future:expr) => {
        async {
            let tic = ::tokio::time::Instant::now();
            let result = $future.await;
            match ::chrono::Duration::from_std(::tokio::time::Instant::now() - tic) {
                Ok(duration) => {
                    ::tracing::event!(::tracing::Level::$level, duration = ?duration, "Operation completed");
                }
                Err(e) => {
                    ::tracing::warn!("Failed to calculate elapsed time: {}", e);
                }
            }
            result
        }
    };

    // Level, future, and message
    ($level:ident, $future:expr; $message:literal) => {
        async {
            let tic = ::tokio::time::Instant::now();
            let result = $future.await;
            match ::chrono::Duration::from_std(::tokio::time::Instant::now() - tic) {
                Ok(duration) => {
                    ::tracing::event!(::tracing::Level::$level, duration = ?duration, $message);
                }
                Err(e) => {
                    ::tracing::warn!("Failed to calculate elapsed time: {}", e);
                }
            }
            result
        }
    };

    // Level, future, and fields (no message)
    ($level:ident, $future:expr; $($field:ident = $value:expr),+ $(,)?) => {
        async {
            let tic = ::tokio::time::Instant::now();
            let result = $future.await;
            match ::chrono::Duration::from_std(::tokio::time::Instant::now() - tic) {
                Ok(duration) => {
                    ::tracing::event!(::tracing::Level::$level, duration = ?duration, $($field = $value),+);
                }
                Err(e) => {
                    ::tracing::warn!("Failed to calculate elapsed time: {}", e);
                }
            }
            result
        }
    };

    // Level, future, message, and fields
    ($level:ident, $future:expr; $($field:ident = $value:expr),+ , $message:literal $(,)?) => {
        async {
            let tic = ::tokio::time::Instant::now();
            let result = $future.await;
            match ::chrono::Duration::from_std(::tokio::time::Instant::now() - tic) {
                Ok(duration) => {
                    ::tracing::event!(::tracing::Level::$level, duration = ?duration, $($field = $value),+, $message);
                }
                Err(e) => {
                    ::tracing::warn!("Failed to calculate elapsed time: {}", e);
                }
            }
            result
        }
    };

    // Result-aware: simple prefix without fat arrow - just level and future
    (with_result($result_var:ident) $level:ident, $future:expr) => {
        async {
            let tic = ::tokio::time::Instant::now();
            let result = $future.await;
            match ::chrono::Duration::from_std(::tokio::time::Instant::now() - tic) {
                Ok(duration) => {
                    let $result_var = &result;
                    ::tracing::event!(::tracing::Level::$level, duration = ?duration, "Operation completed");
                }
                Err(e) => {
                    ::tracing::warn!("Failed to calculate elapsed time: {}", e);
                }
            }
            result
        }
    };

    // Result-aware: simple prefix + level, future, and message
    (with_result($result_var:ident) $level:ident, $future:expr; $message:literal) => {
        async {
            let tic = ::tokio::time::Instant::now();
            let result = $future.await;
            match ::chrono::Duration::from_std(::tokio::time::Instant::now() - tic) {
                Ok(duration) => {
                    let $result_var = &result;
                    ::tracing::event!(::tracing::Level::$level, duration = ?duration, $message);
                }
                Err(e) => {
                    ::tracing::warn!("Failed to calculate elapsed time: {}", e);
                }
            }
            result
        }
    };

    // Result-aware: simple prefix + level, future, and fields (no message)
    (with_result($result_var:ident) $level:ident, $future:expr; $($field:ident = $value:expr),+ $(,)?) => {
        async {
            let tic = ::tokio::time::Instant::now();
            let result = $future.await;
            match ::chrono::Duration::from_std(::tokio::time::Instant::now() - tic) {
                Ok(duration) => {
                    let $result_var = &result;
                    ::tracing::event!(::tracing::Level::$level, duration = ?duration, $($field = $value),+);
                }
                Err(e) => {
                    ::tracing::warn!("Failed to calculate elapsed time: {}", e);
                }
            }
            result
        }
    };

    // Result-aware: simple prefix + level, future, message, and fields
    (with_result($result_var:ident) $level:ident, $future:expr; $($field:ident = $value:expr),+ , $message:literal $(,)?) => {
        async {
            let tic = ::tokio::time::Instant::now();
            let result = $future.await;
            match ::chrono::Duration::from_std(::tokio::time::Instant::now() - tic) {
                Ok(duration) => {
                    let $result_var = &result;
                    ::tracing::event!(::tracing::Level::$level, duration = ?duration, $($field = $value),+, $message);
                }
                Err(e) => {
                    ::tracing::warn!("Failed to calculate elapsed time: {}", e);
                }
            }
            result
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Duration as TokioDuration, sleep};

    #[tokio::test]
    async fn test_tic_toc() {
        let result = tic_toc(async { 42 }).await;
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_timed_event_basic() {
        let result = timed_event!(INFO, async { 42 }).await;
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_timed_event_with_delay() {
        async fn delayed_future() -> i32 {
            sleep(TokioDuration::from_millis(50)).await;
            100
        }
        let result = timed_event!(DEBUG, delayed_future()).await;
        assert_eq!(result, 100);
    }

    #[tokio::test]
    async fn test_timed_event_with_message() {
        let result = timed_event!(INFO, async { "hello" }; "Custom operation").await;
        assert_eq!(result, "hello");
    }

    #[tokio::test]
    async fn test_timed_event_with_fields() {
        let result = timed_event!(WARN, async { 123 }; user_id = 456).await;
        assert_eq!(result, 123);
    }

    #[tokio::test]
    async fn test_timed_event_with_multiple_fields() {
        let result = timed_event!(INFO, async { "success" }; operation = "test", count = 42).await;
        assert_eq!(result, "success");
    }

    #[tokio::test]
    async fn test_timed_event_preserves_error() {
        let result: Result<i32, &str> = timed_event!(INFO, async { Err("test error") }).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "test error");
    }

    #[tokio::test]
    async fn test_timed_event_different_levels() {
        let _trace = timed_event!(TRACE, async { 1 }).await;
        let _debug = timed_event!(DEBUG, async { 2 }).await;
        let _info = timed_event!(INFO, async { 3 }).await;
        let _warn = timed_event!(WARN, async { 4 }).await;
        let _error = timed_event!(ERROR, async { 5 }).await;
    }

    // Test that the macro works with complex futures
    async fn complex_async_function(value: i32) -> Result<String, &'static str> {
        sleep(TokioDuration::from_millis(1)).await;
        if value > 0 {
            Ok(format!("Value: {value}"))
        } else {
            Err("Invalid value")
        }
    }

    #[tokio::test]
    async fn test_timed_event_with_complex_future() {
        let result = timed_event!(INFO, complex_async_function(42); "Complex operation").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Value: 42");
    }

    #[tokio::test]
    async fn test_timed_event_with_complex_future_and_fields() {
        let result = timed_event!(DEBUG, complex_async_function(100); operation = "database_query", query_id = 123)
        .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Value: 100");
    }

    #[tokio::test]
    async fn test_timed_event_with_fields_and_message() {
        let result = timed_event!(INFO, async { "success" }; operation = "test", user_id = 456, "Operation completed successfully").await;
        assert_eq!(result, "success");
    }

    #[tokio::test]
    async fn test_timed_event_error_case() {
        let result = timed_event!(WARN, complex_async_function(-1); "Should fail").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid value");
    }

    // Test result-aware logging
    #[tokio::test]
    async fn test_timed_event_result_aware_basic() {
        let result = timed_event!(with_result(res) INFO, async { 42 }; result_value = *res).await;
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_timed_event_result_aware_with_message() {
        let result = timed_event!(with_result(res) INFO, async { "success".to_string() }; 
                                  result_len = res.len(), "Operation completed")
        .await;
        assert_eq!(result, "success");
    }

    #[tokio::test]
    async fn test_timed_event_result_aware_with_complex_function() {
        let result = timed_event!(with_result(res) INFO, complex_async_function(100); 
                                  operation = "complex", 
                                  success = res.is_ok(), "Complex operation completed")
        .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Value: 100");
    }

    #[tokio::test]
    async fn test_timed_event_result_aware_error_case() {
        let result = timed_event!(with_result(res) WARN, complex_async_function(-1);
                                  operation = "complex",
                                  has_error = res.is_err(), "Complex operation completed")
        .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid value");
    }

    #[tokio::test]
    async fn test_timed_event_result_aware_status_code() {
        async fn get_status() -> u16 {
            404
        }

        let status = timed_event!(with_result(code) WARN, get_status();
                                  status_code = *code, "HTTP request completed")
        .await;
        assert_eq!(status, 404);
    }

    #[tokio::test]
    async fn test_timed_event_result_aware_no_fields() {
        let result = timed_event!(with_result(_items) DEBUG, async { vec![1, 2, 3] }).await;
        assert_eq!(result, vec![1, 2, 3]);
    }
}
