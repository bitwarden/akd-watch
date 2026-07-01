//! HTTP-boundary tests using `mockito`. One behavior per test, asserts on the
//! observable `Result<[u8; 32], VerifyAuditError>` rather than internal state.

use akd_watch_protocol::{
    Epoch, EpochSignature, NamespaceInfo, NamespaceStatus, akd_configurations::AkdConfiguration,
    crypto::SigningKey,
};
use akd_watch_web_client::{AuditOptions, Client, HttpError, VerifyAuditError, VerifyingKey};
use chrono::{Duration, Utc};
use mockito::{Matcher, ServerGuard};
use rand::RngCore;
use serde_json::json;
use uuid::Uuid;

const NAMESPACE: &str = "ns";

struct Fixture {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl Fixture {
    fn new() -> Self {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        let inner = ed25519_dalek::SigningKey::from_bytes(&bytes);
        let key_id = Uuid::new_v4();
        let now = Utc::now();
        let signing = SigningKey::new(inner.clone(), key_id, now, now + Duration::days(30));
        let verifying = VerifyingKey {
            verifying_key: inner.verifying_key(),
            key_id,
            not_before: now,
        };
        Self {
            signing_key: signing,
            verifying_key: verifying,
        }
    }
}

fn key_info_json(key: &VerifyingKey) -> serde_json::Value {
    json!({
        "public_key": hex::encode(key.verifying_key.as_bytes()),
        "key_id": key.key_id.to_string(),
        "not_before": key.not_before.timestamp() as u64,
    })
}

fn info_body(keys: &[VerifyingKey]) -> String {
    json!({ "keys": keys.iter().map(key_info_json).collect::<Vec<_>>() }).to_string()
}

fn namespace_info() -> NamespaceInfo {
    NamespaceInfo {
        configuration: AkdConfiguration::WhatsAppV1Configuration,
        name: NAMESPACE.to_string(),
        log_directory: "log".to_string(),
        last_verified_epoch: None,
        starting_epoch: Epoch::new(0),
        status: NamespaceStatus::Online,
    }
}

fn signature_for(epoch: Epoch, key: &SigningKey) -> (EpochSignature, [u8; 32]) {
    let mut digest = [0u8; 32];
    rand::rng().fill_bytes(&mut digest);
    let sig = EpochSignature::sign(namespace_info(), epoch, digest, key).expect("sign");
    (sig, digest)
}

fn signature_response_json(sig: &EpochSignature) -> serde_json::Value {
    let EpochSignature::V1(v1) = sig;
    let cs: u32 = v1.ciphersuite.into();
    json!({
        "version": sig.version_int(),
        "ciphersuite": cs,
        "namespace": v1.namespace,
        "timestamp": v1.timestamp as u64,
        "epoch": *v1.epoch.value(),
        "digest": hex::encode(&v1.digest),
        "signature": hex::encode(&v1.signature),
        "key_id": v1.key_id.to_string(),
    })
}

async fn server() -> ServerGuard {
    mockito::Server::new_async().await
}

/// Pre-register the namespace-existence probe that `Client::build` performs.
/// Tests that exercise post-build behavior must call this before `build()`.
async fn mock_namespace_exists(srv: &mut ServerGuard) -> mockito::Mock {
    let body = serde_json::to_string(&namespace_info()).expect("serialize NamespaceInfo");
    srv.mock("GET", &*format!("/namespaces/{NAMESPACE}"))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .expect_at_least(1)
        .create_async()
        .await
}

#[tokio::test]
async fn happy_path_returns_audit_root_hash() {
    let mut srv = server().await;
    let fixture = Fixture::new();
    let epoch = Epoch::new(5);
    let (sig, digest) = signature_for(epoch, &fixture.signing_key);

    let _info = srv
        .mock("GET", "/info")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(info_body(&[fixture.verifying_key.clone()]))
        .create_async()
        .await;
    let _audit = srv
        .mock(
            "GET",
            Matcher::Exact(format!("/namespaces/{NAMESPACE}/audits/5")),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(signature_response_json(&sig).to_string())
        .create_async()
        .await;

    let _ns = mock_namespace_exists(&mut srv).await;
    let client = Client::builder(srv.url(), NAMESPACE)
        .pinned_keys(vec![fixture.verifying_key.clone()])
        .build()
        .await
        .expect("build");

    let returned = client
        .verify_audit(epoch, AuditOptions::default())
        .await
        .expect("ok");
    assert_eq!(returned, digest);
}

#[tokio::test]
async fn build_fails_when_server_publishes_imposter_for_pinned_id() {
    let mut srv = server().await;
    let pinned = Fixture::new().verifying_key;
    let mut imposter = Fixture::new().verifying_key;
    imposter.key_id = pinned.key_id;

    let _info = srv
        .mock("GET", "/info")
        .with_status(200)
        .with_body(info_body(&[imposter]))
        .create_async()
        .await;

    let _ns = mock_namespace_exists(&mut srv).await;
    let err = Client::builder(srv.url(), NAMESPACE)
        .pinned_keys(vec![pinned.clone()])
        .build()
        .await
        .expect_err("should fail");
    match err {
        akd_watch_web_client::BuildError::PinnedKeyMissing { key_id } => {
            assert_eq!(key_id, pinned.key_id);
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn audit_signed_by_unpinned_but_published_key_succeeds() {
    // Pinning is build-time only; at audit time the trust set is whatever
    // `/info` publishes. An audit signed by a server-published key the
    // caller did not pin still verifies.
    let mut srv = server().await;
    let pinned = Fixture::new();
    let auditor = Fixture::new();
    let epoch = Epoch::new(2);
    let (sig, digest) = signature_for(epoch, &auditor.signing_key);

    let _info = srv
        .mock("GET", "/info")
        .with_status(200)
        .with_body(info_body(&[
            pinned.verifying_key.clone(),
            auditor.verifying_key.clone(),
        ]))
        .expect_at_least(1)
        .create_async()
        .await;
    let _audit = srv
        .mock(
            "GET",
            Matcher::Exact(format!("/namespaces/{NAMESPACE}/audits/2")),
        )
        .with_status(200)
        .with_body(signature_response_json(&sig).to_string())
        .create_async()
        .await;

    let _ns = mock_namespace_exists(&mut srv).await;
    let client = Client::builder(srv.url(), NAMESPACE)
        .pinned_keys(vec![pinned.verifying_key.clone()])
        .build()
        .await
        .expect("build");

    let returned = client
        .verify_audit(epoch, AuditOptions::default())
        .await
        .expect("verifies against server-published key");
    assert_eq!(returned, digest);
}

#[tokio::test]
async fn audit_signed_by_key_absent_from_info_yields_auditor_signature_invalid() {
    let mut srv = server().await;
    let trusted = Fixture::new();
    let phantom_signer = Fixture::new();
    let epoch = Epoch::new(3);
    let (sig, _digest) = signature_for(epoch, &phantom_signer.signing_key);

    // /info gets hit twice: build-time fetch, and a refresh-on-miss when
    // verify_audit doesn't recognize the audit's key id.
    let _info = srv
        .mock("GET", "/info")
        .with_status(200)
        .with_body(info_body(&[trusted.verifying_key.clone()]))
        .expect_at_least(2)
        .create_async()
        .await;
    let _audit = srv
        .mock(
            "GET",
            Matcher::Exact(format!("/namespaces/{NAMESPACE}/audits/3")),
        )
        .with_status(200)
        .with_body(signature_response_json(&sig).to_string())
        .create_async()
        .await;

    let _ns = mock_namespace_exists(&mut srv).await;
    let client = Client::builder(srv.url(), NAMESPACE)
        .pinned_keys(vec![trusted.verifying_key.clone()])
        .build()
        .await
        .expect("build");

    let err = client
        .verify_audit(epoch, AuditOptions::default())
        .await
        .expect_err("should fail");
    match err {
        VerifyAuditError::AuditorSignatureInvalid {
            key_id, epoch: e, ..
        } => {
            assert_eq!(key_id, phantom_signer.verifying_key.key_id);
            assert_eq!(e, epoch);
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn corrupted_signature_is_auditor_failure() {
    let mut srv = server().await;
    let fixture = Fixture::new();
    let epoch = Epoch::new(4);
    let (sig, _digest) = signature_for(epoch, &fixture.signing_key);

    let mut tampered = signature_response_json(&sig);
    let bad_sig = {
        let EpochSignature::V1(v1) = &sig;
        let mut bytes = v1.signature.clone();
        bytes[0] ^= 0xFF;
        hex::encode(bytes)
    };
    tampered["signature"] = serde_json::Value::String(bad_sig);

    let _info = srv
        .mock("GET", "/info")
        .with_status(200)
        .with_body(info_body(&[fixture.verifying_key.clone()]))
        .create_async()
        .await;
    let _audit = srv
        .mock(
            "GET",
            Matcher::Exact(format!("/namespaces/{NAMESPACE}/audits/4")),
        )
        .with_status(200)
        .with_body(tampered.to_string())
        .create_async()
        .await;

    let _ns = mock_namespace_exists(&mut srv).await;
    let client = Client::builder(srv.url(), NAMESPACE)
        .pinned_keys(vec![fixture.verifying_key.clone()])
        .build()
        .await
        .expect("build");

    let err = client
        .verify_audit(epoch, AuditOptions::default())
        .await
        .expect_err("should fail");
    match err {
        VerifyAuditError::AuditorSignatureInvalid {
            namespace,
            epoch: e,
            ..
        } => {
            assert_eq!(namespace, NAMESPACE);
            assert_eq!(e, epoch);
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn expected_root_hash_mismatch_is_unique_error() {
    let mut srv = server().await;
    let fixture = Fixture::new();
    let epoch = Epoch::new(6);
    let (sig, actual) = signature_for(epoch, &fixture.signing_key);
    let mut wrong = actual;
    wrong[0] ^= 0x01;

    let _info = srv
        .mock("GET", "/info")
        .with_status(200)
        .with_body(info_body(&[fixture.verifying_key.clone()]))
        .create_async()
        .await;
    let _audit = srv
        .mock(
            "GET",
            Matcher::Exact(format!("/namespaces/{NAMESPACE}/audits/6")),
        )
        .with_status(200)
        .with_body(signature_response_json(&sig).to_string())
        .create_async()
        .await;

    let _ns = mock_namespace_exists(&mut srv).await;
    let client = Client::builder(srv.url(), NAMESPACE)
        .pinned_keys(vec![fixture.verifying_key.clone()])
        .build()
        .await
        .expect("build");

    let err = client
        .verify_audit(
            epoch,
            AuditOptions {
                expected_root_hash: Some(wrong),
            },
        )
        .await
        .expect_err("should fail");
    match err {
        VerifyAuditError::RootHashMismatch {
            namespace,
            epoch: e,
            expected_hex,
            actual_hex,
        } => {
            assert_eq!(namespace, NAMESPACE);
            assert_eq!(e, epoch);
            assert_eq!(expected_hex, hex::encode(wrong));
            assert_eq!(actual_hex, hex::encode(actual));
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn server_404_for_audit_yields_audit_not_available() {
    let mut srv = server().await;
    let fixture = Fixture::new();

    let _info = srv
        .mock("GET", "/info")
        .with_status(200)
        .with_body(info_body(&[fixture.verifying_key.clone()]))
        .create_async()
        .await;
    let _audit = srv
        .mock(
            "GET",
            Matcher::Exact(format!("/namespaces/{NAMESPACE}/audits/9")),
        )
        .with_status(404)
        .create_async()
        .await;

    let _ns = mock_namespace_exists(&mut srv).await;
    let client = Client::builder(srv.url(), NAMESPACE)
        .pinned_keys(vec![fixture.verifying_key.clone()])
        .build()
        .await
        .expect("build");

    let err = client
        .verify_audit(Epoch::new(9), AuditOptions::default())
        .await
        .expect_err("should fail");
    match err {
        VerifyAuditError::AuditNotAvailable { namespace, epoch } => {
            assert_eq!(namespace, NAMESPACE);
            assert_eq!(epoch, Epoch::new(9));
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn server_returns_null_audit_yields_audit_not_available() {
    let mut srv = server().await;
    let fixture = Fixture::new();

    let _info = srv
        .mock("GET", "/info")
        .with_status(200)
        .with_body(info_body(&[fixture.verifying_key.clone()]))
        .create_async()
        .await;
    let _audit = srv
        .mock(
            "GET",
            Matcher::Exact(format!("/namespaces/{NAMESPACE}/audits/12")),
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("null")
        .create_async()
        .await;

    let _ns = mock_namespace_exists(&mut srv).await;
    let client = Client::builder(srv.url(), NAMESPACE)
        .pinned_keys(vec![fixture.verifying_key.clone()])
        .build()
        .await
        .expect("build");

    let err = client
        .verify_audit(Epoch::new(12), AuditOptions::default())
        .await
        .expect_err("should fail");
    assert!(matches!(err, VerifyAuditError::AuditNotAvailable { .. }));
}

#[tokio::test]
async fn audit_endpoint_400_after_build_surfaces_as_protocol_mismatch() {
    // The build-time namespace probe guarantees the namespace exists when
    // we hold a Client. If the audit endpoint later returns 400 (e.g.
    // server-side state drifted, or version mismatch), it surfaces as
    // ProtocolMismatch — distinct from a 5xx Unavailable because the
    // operator action is "align versions / investigate", not "wait & retry".
    let mut srv = server().await;
    let fixture = Fixture::new();

    let _info = srv
        .mock("GET", "/info")
        .with_status(200)
        .with_body(info_body(&[fixture.verifying_key.clone()]))
        .create_async()
        .await;
    let _audit = srv
        .mock(
            "GET",
            Matcher::Exact(format!("/namespaces/{NAMESPACE}/audits/1")),
        )
        .with_status(400)
        .with_body("namespace ns not found")
        .create_async()
        .await;

    let _ns = mock_namespace_exists(&mut srv).await;
    let client = Client::builder(srv.url(), NAMESPACE)
        .pinned_keys(vec![fixture.verifying_key.clone()])
        .build()
        .await
        .expect("build");

    let err = client
        .verify_audit(Epoch::new(1), AuditOptions::default())
        .await
        .expect_err("should fail");
    match err {
        VerifyAuditError::Http(HttpError::ProtocolMismatch { reason }) => {
            assert!(
                reason.contains("400"),
                "expected reason to mention status 400: {reason}"
            );
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn build_fails_with_protocol_mismatch_for_malformed_info_json() {
    // /info is fetched at build time, so wire-format failures surface as
    // BuildError, not VerifyAuditError.
    let mut srv = server().await;

    let _info = srv
        .mock("GET", "/info")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("{ not even json")
        .create_async()
        .await;

    let _ns = mock_namespace_exists(&mut srv).await;
    let err = Client::builder(srv.url(), NAMESPACE)
        .build()
        .await
        .expect_err("should fail");
    assert!(matches!(
        err,
        akd_watch_web_client::BuildError::Http(HttpError::ProtocolMismatch { .. })
    ));
}

#[tokio::test]
async fn unpinned_client_verifies_against_server_keys() {
    let mut srv = server().await;
    let fixture = Fixture::new();
    let epoch = Epoch::new(11);
    let (sig, digest) = signature_for(epoch, &fixture.signing_key);

    let _info = srv
        .mock("GET", "/info")
        .with_status(200)
        .with_body(info_body(&[fixture.verifying_key.clone()]))
        .create_async()
        .await;
    let _audit = srv
        .mock(
            "GET",
            Matcher::Exact(format!("/namespaces/{NAMESPACE}/audits/11")),
        )
        .with_status(200)
        .with_body(signature_response_json(&sig).to_string())
        .create_async()
        .await;

    // No pinned keys -> trust whatever /info publishes.
    let _ns = mock_namespace_exists(&mut srv).await;
    let client = Client::builder(srv.url(), NAMESPACE)
        .build()
        .await
        .expect("build");

    let returned = client
        .verify_audit(epoch, AuditOptions::default())
        .await
        .expect("ok");
    assert_eq!(returned, digest);
}

#[tokio::test]
async fn refresh_on_miss_resolves_freshly_added_key() {
    let mut srv = server().await;
    let fixture = Fixture::new();
    let epoch = Epoch::new(13);
    let (sig, digest) = signature_for(epoch, &fixture.signing_key);

    // First /info: empty key set. Second /info: contains the signing key.
    let _info_1 = srv
        .mock("GET", "/info")
        .with_status(200)
        .with_body(info_body(&[]))
        .expect(1)
        .create_async()
        .await;
    let _info_2 = srv
        .mock("GET", "/info")
        .with_status(200)
        .with_body(info_body(&[fixture.verifying_key.clone()]))
        .expect_at_least(1)
        .create_async()
        .await;
    let _audit = srv
        .mock(
            "GET",
            Matcher::Exact(format!("/namespaces/{NAMESPACE}/audits/13")),
        )
        .with_status(200)
        .with_body(signature_response_json(&sig).to_string())
        .create_async()
        .await;

    // No pinning -> the first /info (an empty set) is accepted as the trusted
    // set, but the audit's key id is not in it, so the client refreshes /info
    // and finds the key on the second fetch.
    let _ns = mock_namespace_exists(&mut srv).await;
    let client = Client::builder(srv.url(), NAMESPACE)
        .build()
        .await
        .expect("build");

    let returned = client
        .verify_audit(epoch, AuditOptions::default())
        .await
        .expect("ok");
    assert_eq!(returned, digest);
}

// -- build-time error surface --

#[tokio::test]
async fn build_returns_namespace_not_found_when_server_404s() {
    let mut srv = server().await;

    let _ns = srv
        .mock("GET", &*format!("/namespaces/{NAMESPACE}"))
        .with_status(404)
        .create_async()
        .await;

    let err = Client::builder(srv.url(), NAMESPACE)
        .build()
        .await
        .expect_err("should fail");
    match err {
        akd_watch_web_client::BuildError::NamespaceNotFound { namespace } => {
            assert_eq!(namespace, NAMESPACE);
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[tokio::test]
async fn build_returns_namespace_not_found_when_server_returns_null() {
    let mut srv = server().await;

    let _ns = srv
        .mock("GET", &*format!("/namespaces/{NAMESPACE}"))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("null")
        .create_async()
        .await;

    let err = Client::builder(srv.url(), NAMESPACE)
        .build()
        .await
        .expect_err("should fail");
    assert!(matches!(
        err,
        akd_watch_web_client::BuildError::NamespaceNotFound { .. }
    ));
}

#[tokio::test]
async fn build_returns_invalid_base_url_for_unsupported_scheme() {
    let err = Client::builder("ftp://example.com", NAMESPACE)
        .build()
        .await
        .expect_err("should fail");
    assert!(matches!(
        err,
        akd_watch_web_client::BuildError::InvalidBaseUrl { .. }
    ));
}

#[tokio::test]
async fn build_returns_empty_namespace_for_blank_namespace() {
    let err = Client::builder("https://example.com", "")
        .build()
        .await
        .expect_err("should fail");
    assert!(matches!(
        err,
        akd_watch_web_client::BuildError::EmptyNamespace
    ));
}

#[tokio::test]
async fn build_returns_http_unavailable_on_5xx() {
    let mut srv = server().await;

    let _ns = srv
        .mock("GET", &*format!("/namespaces/{NAMESPACE}"))
        .with_status(503)
        .with_body("backend down")
        .create_async()
        .await;

    let err = Client::builder(srv.url(), NAMESPACE)
        .build()
        .await
        .expect_err("should fail");
    match err {
        akd_watch_web_client::BuildError::Http(HttpError::Unavailable { reason }) => {
            assert!(
                reason.contains("503"),
                "expected reason to mention status 503: {reason}"
            );
        }
        other => panic!("unexpected: {other:?}"),
    }
}
