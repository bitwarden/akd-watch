use akd_watch_protocol::{
    Epoch, NamespaceInfo,
    crypto::VerifyingKey,
    timed_event,
    web_api::{KeyInfo, ServerConfiguration, SignatureResponse},
};
use reqwest::StatusCode;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};
use url::Url;

use crate::{
    error::{BuildError, HttpError, VerifyAuditError},
    keys::{find_in_server, validate_pinned_against_server},
    verify::verify_signature,
};

/// Per-call options for [`Client::verify_audit`].
#[derive(Default, Clone, Debug)]
pub struct AuditOptions {
    /// If set, the audit's epoch root hash digest must equal these bytes.
    /// A mismatch produces [`VerifyAuditError::RootHashMismatch`] — distinct
    /// from any auditor or transport error so callers can treat it as a
    /// possible split-world signal.
    pub expected_root_hash: Option<[u8; 32]>,
}

/// A verifying client bound to a single akd-watch instance and namespace.
///
/// Constructing a `Client` requires successful HTTP probes of the namespace
/// and `/info`: once you hold a `Client`, the namespace existed on the
/// akd-watch and any caller-supplied pinned keys were present in the
/// server's published key set. If the server later removes or
/// reconfigures the namespace, that change will surface as
/// [`HttpError::ProtocolMismatch`] on the next call.
///
/// At runtime, the trusted verifying-key set is whatever `/info` publishes;
/// pinning is enforced only at build time.
///
/// To audit a different namespace or a different akd-watch, build another
/// [`Client`].
#[derive(Debug)]
pub struct Client {
    base_url: Url,
    namespace: String,
    http: reqwest::Client,
    server_keys: RwLock<Vec<VerifyingKey>>,
}

/// Builder for [`Client`]. Construct with [`Client::builder`].
#[derive(Debug)]
pub struct ClientBuilder {
    base_url: String,
    namespace: String,
    pinned: Option<Vec<VerifyingKey>>,
    http: Option<reqwest::Client>,
}

impl Client {
    /// Begin building a client bound to `base_url` and `namespace`.
    pub fn builder(base_url: impl AsRef<str>, namespace: impl Into<String>) -> ClientBuilder {
        ClientBuilder {
            base_url: base_url.as_ref().to_string(),
            namespace: namespace.into(),
            pinned: None,
            http: None,
        }
    }

    /// Verify the audit signature published for `epoch` and return the
    /// 32-byte epoch root hash.
    ///
    /// The audit must be signed by a key published on the akd-watch's
    /// `/info` endpoint. If the audit references a key we have not yet
    /// seen (because the server rotated since this client was built), the
    /// client refreshes `/info` once before deciding the audit is invalid.
    #[instrument(level = "info", skip(self, opts), fields(namespace = %self.namespace, epoch = %epoch))]
    pub async fn verify_audit(
        &self,
        epoch: Epoch,
        opts: AuditOptions,
    ) -> Result<[u8; 32], VerifyAuditError> {
        let response = self.fetch_audit(epoch).await?;
        let signature = response.into_epoch_signature().map_err(|e| {
            warn!(error = %e, "could not parse audit signature response");
            VerifyAuditError::Http(HttpError::from(e))
        })?;

        let audit_key_id = signature.signing_key_id();
        let key = match self.lookup_audit_key(audit_key_id).await {
            Some(k) => k,
            None => {
                self.refresh_server_keys().await?;
                match self.lookup_audit_key(audit_key_id).await {
                    Some(k) => k,
                    None => {
                        warn!(
                            key_id = %audit_key_id,
                            "audit references key id not present on /info even after refresh — auditor may be using an unpublished key"
                        );
                        return Err(VerifyAuditError::AuditorSignatureInvalid {
                            namespace: self.namespace.clone(),
                            epoch,
                            key_id: audit_key_id,
                        });
                    }
                }
            }
        };

        verify_signature(&signature, &key, &self.namespace, epoch)?;

        let digest_len = signature.digest().len();
        let digest = signature.epoch_root_hash().map_err(|_| {
            warn!(
                digest_len,
                "audit digest is not 32 bytes — protocol mismatch"
            );
            HttpError::ProtocolMismatch {
                reason: format!("audit digest length is not 32 bytes: got {digest_len}"),
            }
        })?;

        if let Some(expected) = opts.expected_root_hash
            && expected != digest
        {
            let expected_hex = hex::encode(expected);
            let actual_hex = hex::encode(digest);
            error!(
                namespace = %self.namespace,
                epoch = %epoch,
                expected = %expected_hex,
                actual = %actual_hex,
                "root hash mismatch — possible split-world attack"
            );
            return Err(VerifyAuditError::RootHashMismatch {
                namespace: self.namespace.clone(),
                epoch,
                expected_hex,
                actual_hex,
            });
        }

        info!(key_id = %audit_key_id, "audit verified");
        Ok(digest)
    }

    /// Information about this client's bound namespace. Re-fetches each
    /// call — fields like `last_verified_epoch` advance as the auditor
    /// runs.
    #[instrument(level = "debug", skip(self), fields(namespace = %self.namespace))]
    pub async fn namespace_info(&self) -> Result<NamespaceInfo, HttpError> {
        let url = self.endpoint(&["namespaces", &self.namespace]);
        let response = self.http.get(url).send().await.map_err(transport_error)?;
        let response = check_status(response).await?;
        let info: Option<NamespaceInfo> = response.json().await.map_err(decode_error)?;
        info.ok_or_else(|| {
            warn!(
                namespace = %self.namespace,
                "akd-watch returned null for a namespace that was validated at build time — server-side state may have drifted"
            );
            HttpError::ProtocolMismatch {
                reason: format!(
                    "akd-watch returned null for namespace '{}', which was validated at build time",
                    self.namespace
                ),
            }
        })
    }

    /// All namespaces published by the bound akd-watch.
    #[instrument(level = "debug", skip(self))]
    pub async fn list_namespaces(&self) -> Result<Vec<NamespaceInfo>, HttpError> {
        let url = self.endpoint(&["namespaces"]);
        let response = self.http.get(url).send().await.map_err(transport_error)?;
        let response = check_status(response).await?;
        response.json().await.map_err(decode_error)
    }

    /// Verifying keys currently published by the akd-watch's `/info` endpoint.
    /// Re-fetches each call.
    #[instrument(level = "debug", skip(self))]
    pub async fn server_keys(&self) -> Result<Vec<VerifyingKey>, HttpError> {
        let dto = self.fetch_info().await?;
        dto.keys
            .into_iter()
            .map(KeyInfo::into_verifying_key)
            .collect::<Result<Vec<_>, _>>()
            .map_err(log_key_parse_error)
    }

    #[instrument(level = "debug", skip(self))]
    async fn fetch_info(&self) -> Result<ServerConfiguration, HttpError> {
        let url = self.endpoint(&["info"]);
        let response = timed_event!(DEBUG, self.http.get(url).send(); "/info request")
            .await
            .map_err(transport_error)?;
        let response = check_status(response).await?;
        response.json().await.map_err(decode_error)
    }

    #[instrument(
        level = "debug",
        skip(self),
        fields(namespace = %self.namespace, epoch = %epoch)
    )]
    async fn fetch_audit(&self, epoch: Epoch) -> Result<SignatureResponse, VerifyAuditError> {
        let epoch_str = epoch.to_string();
        let url = self.endpoint(&["namespaces", &self.namespace, "audits", &epoch_str]);
        let response = timed_event!(DEBUG, self.http.get(url).send(); "audit endpoint request")
            .await
            .map_err(transport_error)?;
        let status = response.status();
        if !status.is_success() {
            // 404 means the auditor has no signature for this epoch yet.
            // Everything else hands off to `non_success_error`, which maps
            // 5xx to Unavailable (retry) and other 4xx to ProtocolMismatch
            // (likely a server-side namespace deletion or version drift).
            return Err(match status {
                StatusCode::NOT_FOUND => {
                    info!("audit signature not yet available for this epoch");
                    VerifyAuditError::AuditNotAvailable {
                        namespace: self.namespace.clone(),
                        epoch,
                    }
                }
                _ => {
                    let body = response.text().await.unwrap_or_default();
                    VerifyAuditError::Http(non_success_error(status, body))
                }
            });
        }
        let body: Option<SignatureResponse> = response.json().await.map_err(decode_error)?;
        body.ok_or_else(|| {
            info!("audit endpoint returned null — signature not yet available for this epoch");
            VerifyAuditError::AuditNotAvailable {
                namespace: self.namespace.clone(),
                epoch,
            }
        })
    }

    #[instrument(level = "debug", skip(self))]
    async fn refresh_server_keys(&self) -> Result<(), VerifyAuditError> {
        debug!("refreshing server key set after audit referenced an unknown key id");
        let dto = self.fetch_info().await?;
        let keys: Vec<VerifyingKey> = dto
            .keys
            .into_iter()
            .map(KeyInfo::into_verifying_key)
            .collect::<Result<Vec<_>, _>>()
            .map_err(log_key_parse_error)?;
        let mut state = self.server_keys.write().await;
        *state = keys;
        Ok(())
    }

    async fn lookup_audit_key(&self, audit_key_id: uuid::Uuid) -> Option<VerifyingKey> {
        let state = self.server_keys.read().await;
        find_in_server(audit_key_id, &state)
    }

    /// Build an endpoint URL by appending percent-encoded path segments to
    /// the base URL. Infallible: `base_url` is validated as http/https at
    /// build time, which guarantees `path_segments_mut` returns `Ok`, and
    /// `push` percent-encodes any string into a valid path segment.
    fn endpoint(&self, segments: &[&str]) -> Url {
        let mut url = self.base_url.clone();
        {
            let mut path = url
                .path_segments_mut()
                .expect("base_url has http/https scheme, validated at build time");
            // The base URL ends in `/` (enforced at build time), which leaves
            // an empty trailing segment that `push` would compound. `pop_if_empty`
            // discards it before we append.
            path.pop_if_empty();
            for seg in segments {
                path.push(seg);
            }
        }
        url
    }
}

impl ClientBuilder {
    /// Pin verifying keys the caller expects the akd-watch to publish.
    /// May be called multiple times; keys accumulate. Build will fail if
    /// any pinned key is not in the server's `/info` set, or differs from
    /// the server's bytes for the same id.
    ///
    /// Pinning has no runtime effect beyond this build-time check — the
    /// trusted set during audit verification is whatever `/info` publishes.
    pub fn pinned_keys(mut self, keys: Vec<VerifyingKey>) -> Self {
        match self.pinned.as_mut() {
            Some(existing) => existing.extend(keys),
            None => self.pinned = Some(keys),
        }
        self
    }

    /// Use a pre-configured reqwest client (proxies, custom TLS, timeouts).
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http = Some(client);
        self
    }

    /// Finalize the builder. Validates local configuration, then probes the
    /// akd-watch to confirm the namespace exists and that any pinned keys
    /// are present in the server's published set.
    #[instrument(level = "info", skip(self), fields(namespace = %self.namespace))]
    pub async fn build(self) -> Result<Client, BuildError> {
        if self.namespace.is_empty() {
            warn!("client build rejected: namespace is empty");
            return Err(BuildError::EmptyNamespace);
        }
        let base_url = normalize_base_url(&self.base_url)?;
        let http = self.http.unwrap_or_default();

        probe_namespace(&http, &base_url, &self.namespace).await?;
        let server_keys = fetch_server_keys(&http, &base_url).await?;

        if let Some(pinned) = self.pinned.as_deref() {
            validate_pinned_against_server(pinned, &server_keys)?;
        }

        info!(
            base_url = %base_url,
            key_count = server_keys.len(),
            "akd-watch web client built"
        );

        Ok(Client {
            base_url,
            namespace: self.namespace,
            http,
            server_keys: RwLock::new(server_keys),
        })
    }
}

#[instrument(level = "debug", skip(http))]
async fn probe_namespace(
    http: &reqwest::Client,
    base_url: &Url,
    namespace: &str,
) -> Result<(), BuildError> {
    let mut url = base_url.clone();
    {
        let mut path = url
            .path_segments_mut()
            .expect("base_url scheme validated as http/https");
        path.pop_if_empty();
        path.push("namespaces");
        path.push(namespace);
    }

    let response = timed_event!(DEBUG, http.get(url).send(); "namespace probe request")
        .await
        .map_err(transport_error)?;
    let status = response.status();
    if status == StatusCode::NOT_FOUND {
        warn!(namespace, "akd-watch reports no such namespace (404)");
        return Err(BuildError::NamespaceNotFound {
            namespace: namespace.to_string(),
        });
    }
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(BuildError::Http(non_success_error(status, body)));
    }
    let info: Option<NamespaceInfo> = response.json().await.map_err(decode_error)?;
    if info.is_none() {
        warn!(namespace, "akd-watch returned null for namespace probe");
        return Err(BuildError::NamespaceNotFound {
            namespace: namespace.to_string(),
        });
    }
    Ok(())
}

#[instrument(level = "debug", skip(http))]
async fn fetch_server_keys(
    http: &reqwest::Client,
    base_url: &Url,
) -> Result<Vec<VerifyingKey>, BuildError> {
    let mut url = base_url.clone();
    {
        let mut path = url
            .path_segments_mut()
            .expect("base_url scheme validated as http/https");
        path.pop_if_empty();
        path.push("info");
    }
    let response = timed_event!(DEBUG, http.get(url).send(); "build-time /info request")
        .await
        .map_err(transport_error)?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(BuildError::Http(non_success_error(status, body)));
    }
    let dto: ServerConfiguration = response.json().await.map_err(decode_error)?;
    let keys = dto
        .keys
        .into_iter()
        .map(KeyInfo::into_verifying_key)
        .collect::<Result<Vec<_>, _>>()
        .map_err(log_key_parse_error)?;
    Ok(keys)
}

fn normalize_base_url(raw: &str) -> Result<Url, BuildError> {
    let mut url = Url::parse(raw).map_err(|e| {
        warn!(error = %e, "client build rejected: base URL did not parse");
        BuildError::InvalidBaseUrl {
            reason: e.to_string(),
        }
    })?;
    if !matches!(url.scheme(), "http" | "https") {
        let scheme = url.scheme().to_string();
        warn!(
            scheme,
            "client build rejected: base URL scheme is not http/https"
        );
        return Err(BuildError::InvalidBaseUrl {
            reason: format!("scheme must be http or https, got {scheme}"),
        });
    }
    if !url.path().ends_with('/') {
        let new_path = format!("{}/", url.path());
        url.set_path(&new_path);
    }
    Ok(url)
}

async fn check_status(response: reqwest::Response) -> Result<reqwest::Response, HttpError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().await.unwrap_or_default();
    Err(non_success_error(status, body))
}

/// Map a non-2xx response to the right `HttpError` variant: 5xx becomes
/// `Unavailable` (retry-able outage), everything else becomes
/// `ProtocolMismatch` (version/schema problem). Logs a WARN at the throw
/// site with status and body length so the breadcrumb survives even if the
/// caller does not inspect the error.
fn non_success_error(status: StatusCode, body: String) -> HttpError {
    let code = status.as_u16();
    if status.is_server_error() {
        warn!(
            status = code,
            body_len = body.len(),
            "akd-watch returned 5xx — service unavailable, retry"
        );
        HttpError::Unavailable {
            reason: format!("akd-watch returned status {code}: {body}"),
        }
    } else {
        warn!(
            status = code,
            body_len = body.len(),
            "akd-watch returned non-2xx other than 5xx — likely protocol/version mismatch"
        );
        HttpError::ProtocolMismatch {
            reason: format!("akd-watch returned status {code}: {body}"),
        }
    }
}

/// Map a `reqwest::Error` from a JSON-decode step into the right `HttpError`
/// variant, logging the underlying detail at WARN before returning. Decode
/// failures collapse to `ProtocolMismatch` (we got bytes but cannot read
/// them); transport-side failures during body read collapse to
/// `Unavailable` (the connection broke mid-read).
fn decode_error(e: reqwest::Error) -> HttpError {
    if e.is_decode() {
        warn!(error = %e, "could not decode akd-watch response body — protocol mismatch");
        HttpError::ProtocolMismatch {
            reason: format!("could not decode response body: {e}"),
        }
    } else {
        warn!(error = %e, "transport error reading akd-watch response body — service unavailable");
        HttpError::Unavailable {
            reason: format!("transport error reading body: {e}"),
        }
    }
}

/// Map a `reqwest::Error` from `send().await` into `HttpError::Unavailable`,
/// logging at WARN. Used at every send site so transport failures always
/// leave a log breadcrumb at the throw site.
fn transport_error(e: reqwest::Error) -> HttpError {
    warn!(error = %e, "HTTP transport error talking to akd-watch — service unavailable");
    HttpError::Unavailable {
        reason: format!("transport error: {e}"),
    }
}

/// Log and forward a verifying-key parse error coming back from
/// `KeyInfo::into_verifying_key`. The error already names which field was
/// malformed; we just attach a log line at the call site so the breadcrumb
/// shows up close to the operation that hit it.
fn log_key_parse_error(e: akd_watch_protocol::web_api::WireError) -> HttpError {
    warn!(error = %e, "could not parse a verifying key entry from /info");
    e.into()
}
