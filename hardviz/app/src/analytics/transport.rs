//! Batch transport.
//!
//! A flush never retries inside itself: a batch that could not be delivered
//! waits, unchanged, for the next flush point.

use std::time::Duration;

use super::batch::SealedBatch;

/// The only endpoint a release build uses. No setting, environment
/// variable, or remote switch changes it.
const ENDPOINT: &str = "https://analytics.hardviz.com/v1/batches";
/// Debug builds only: a loopback URL that makes them upload to a local
/// `wrangler dev` instead of logging. See [`debug_endpoint`].
const DEBUG_ENDPOINT_VAR: &str = "HARDVIZ_ANALYTICS_DEBUG_ENDPOINT";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// What a send attempt means for the head batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Delivery {
  /// Stored by the service; the queue advances.
  Accepted,
  /// Terminally refused: nothing in a retry changes a rejected shape or
  /// day, so the batch is dropped and the queue advances.
  Rejected,
  /// A network error, a timeout, `429`, or `5xx`: the batch stays at the
  /// head for the next flush.
  Deferred,
}

impl Delivery {
  pub(crate) fn from_status(status: u16) -> Self {
    match status {
      200..=299 => Self::Accepted,
      429 | 500..=599 => Self::Deferred,
      400..=499 => Self::Rejected,
      // Neither accepted nor refused by the Worker's contract; keep the
      // batch rather than drop data on an unexpected answer.
      _ => Self::Deferred,
    }
  }
}

#[async_trait::async_trait]
pub(crate) trait Transport: Send + Sync + 'static {
  async fn send(&self, batch: &SealedBatch) -> Delivery;
}

/// Uploads over HTTPS with `reqwest`.
pub(crate) struct HttpTransport {
  client: reqwest::Client,
  endpoint: String,
}

impl HttpTransport {
  /// reqwest is compiled with `rustls-no-provider`, which does not pick a
  /// provider from crate features, so a client built before a default
  /// `CryptoProvider` exists panics. The updater installs its own only when
  /// used, so this boundary installs `ring` the way
  /// `external_component_setup::download_client` does. A second
  /// installation returns `Err`, which is the already-installed case.
  pub(crate) fn new(endpoint: String) -> Result<Self, reqwest::Error> {
    Self::build(endpoint, reqwest::Client::builder())
  }

  /// The client for a [`debug_endpoint`]. The loopback check covers only the
  /// configured URL, so nothing may carry the request elsewhere: a system
  /// proxy (`HTTP_PROXY` does not exempt loopback) would send the batch off
  /// the machine, and a redirect could point anywhere. A `3xx` answer is
  /// therefore not followed and defers the batch like any unexpected status.
  fn loopback(endpoint: String) -> Result<Self, reqwest::Error> {
    Self::build(
      endpoint,
      reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none()),
    )
  }

  fn build(
    endpoint: String,
    builder: reqwest::ClientBuilder,
  ) -> Result<Self, reqwest::Error> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = builder
      .timeout(REQUEST_TIMEOUT)
      .user_agent(concat!("HardwareVisualizer/", env!("CARGO_PKG_VERSION")))
      .build()?;
    Ok(Self { client, endpoint })
  }
}

#[async_trait::async_trait]
impl Transport for HttpTransport {
  async fn send(&self, batch: &SealedBatch) -> Delivery {
    let body = match serde_json::to_vec(batch) {
      Ok(body) => body,
      Err(error) => {
        crate::log_error!(
          "Failed to serialize a product analytics batch",
          "analytics::HttpTransport::send",
          Some(error.to_string())
        );
        return Delivery::Rejected;
      }
    };
    let response = self
      .client
      .post(&self.endpoint)
      .header(reqwest::header::CONTENT_TYPE, "application/json")
      .body(body)
      .send()
      .await;
    match response {
      Ok(response) => {
        let delivery = Delivery::from_status(response.status().as_u16());
        if delivery != Delivery::Accepted {
          crate::log_debug!(
            &format!("Product analytics batch answered {}", response.status()),
            "analytics::HttpTransport::send",
            None::<&str>
          );
        }
        delivery
      }
      Err(error) => {
        // The error text is logged without the URL or payload.
        crate::log_debug!(
          "Product analytics upload failed",
          "analytics::HttpTransport::send",
          Some(error.without_url().to_string())
        );
        Delivery::Deferred
      }
    }
  }
}

/// Debug builds never upload to the service: development data would pollute
/// the dataset, and the client-service contract is tested through example
/// payloads, not a live call. This mirrors `fetch_update`, which skips the
/// updater in debug builds. The one exception is a loopback
/// [`debug_endpoint`], which reaches only a Worker on the same machine.
pub(crate) struct LoggingTransport;

#[async_trait::async_trait]
impl Transport for LoggingTransport {
  async fn send(&self, batch: &SealedBatch) -> Delivery {
    crate::log_info!(
      "Debug build: product analytics batch not uploaded",
      "analytics::LoggingTransport::send",
      serde_json::to_string(batch).ok()
    );
    Delivery::Accepted
  }
}

/// The transport this build uses.
pub(crate) fn build_transport() -> Result<std::sync::Arc<dyn Transport>, String> {
  let transport = if cfg!(debug_assertions) {
    match debug_endpoint(std::env::var(DEBUG_ENDPOINT_VAR).ok().as_deref()) {
      Some(endpoint) => HttpTransport::loopback(endpoint),
      None => return Ok(std::sync::Arc::new(LoggingTransport)),
    }
  } else {
    HttpTransport::new(ENDPOINT.to_string())
  };
  transport
    .map(|transport| std::sync::Arc::new(transport) as std::sync::Arc<dyn Transport>)
    .map_err(|error| format!("Failed to build the product analytics client: {error}"))
}

/// The debug-build upload target, if [`DEBUG_ENDPOINT_VAR`] names one on a
/// loopback host. Any other host is refused, so a debug build can reach a
/// local `wrangler dev` but never the production service or a third party.
/// Release builds never call this.
fn debug_endpoint(value: Option<&str>) -> Option<String> {
  let value = value?.trim();
  if value.is_empty() {
    return None;
  }
  let loopback = reqwest::Url::parse(value).ok().is_some_and(|url| {
    matches!(url.scheme(), "http" | "https")
      && url.host_str().is_some_and(|host| {
        host.eq_ignore_ascii_case("localhost")
          || host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
      })
  });
  if !loopback {
    crate::log_warn!(
      "Ignoring a non-loopback product analytics debug endpoint",
      "analytics::debug_endpoint",
      None::<&str>
    );
    return None;
  }
  Some(value.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn statuses_map_to_the_retry_policy() {
    for status in [200, 202] {
      assert_eq!(Delivery::from_status(status), Delivery::Accepted);
    }
    for status in [429, 500, 502, 503] {
      assert_eq!(
        Delivery::from_status(status),
        Delivery::Deferred,
        "{status}"
      );
    }
    for status in [400, 401, 404, 413, 422] {
      assert_eq!(
        Delivery::from_status(status),
        Delivery::Rejected,
        "{status}"
      );
    }
  }

  #[test]
  fn the_endpoint_is_the_fixed_https_service() {
    assert_eq!(ENDPOINT, "https://analytics.hardviz.com/v1/batches");
    assert_eq!(REQUEST_TIMEOUT, Duration::from_secs(10));
  }

  #[test]
  fn the_release_client_builds_with_the_ring_provider() {
    assert!(HttpTransport::new(ENDPOINT.to_string()).is_ok());
  }

  /// The loopback check covers only the configured URL, so a redirect from
  /// it, which could point at the production service, must not be followed.
  /// The target here is a second local listener that must see no
  /// connection.
  #[tokio::test]
  async fn the_debug_client_does_not_follow_redirects() {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let target = TcpListener::bind("127.0.0.1:0").unwrap();
    target.set_nonblocking(true).unwrap();
    let target_url = format!("http://{}/v1/batches", target.local_addr().unwrap());

    let redirecting = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/v1/batches", redirecting.local_addr().unwrap());
    let server = std::thread::spawn(move || {
      let (mut stream, _) = redirecting.accept().unwrap();
      let mut request = [0_u8; 4096];
      let _ = stream.read(&mut request);
      let response = format!(
        "HTTP/1.1 307 Temporary Redirect\r\nLocation: {target_url}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
      );
      stream.write_all(response.as_bytes()).unwrap();
    });

    let mut aggregate = crate::analytics::batch::LocalAggregate::default();
    let mut consent = crate::analytics::consent::GrantedConsent::from_settings(
      &crate::analytics::consent::test_support::granted_section(),
    )
    .unwrap();
    aggregate.seal(&mut consent, chrono::Utc::now(), None, |_| Ok(()));
    let batch = aggregate.head().unwrap();

    let transport = HttpTransport::loopback(endpoint).unwrap();
    assert_eq!(transport.send(batch).await, Delivery::Deferred);
    server.join().unwrap();
    assert_eq!(
      target.accept().map_err(|error| error.kind()).err(),
      Some(std::io::ErrorKind::WouldBlock),
      "the redirect target must not be contacted"
    );
  }

  /// A debug build may upload only to its own machine, so an override can
  /// never send development data to the production service.
  #[test]
  fn the_debug_endpoint_accepts_only_loopback_hosts() {
    for local in [
      "http://localhost:8787/v1/batches",
      "http://127.0.0.1:8787/v1/batches",
      "http://[::1]:8787/v1/batches",
    ] {
      assert_eq!(debug_endpoint(Some(local)).as_deref(), Some(local));
    }
    for refused in [
      None,
      Some(""),
      Some("https://analytics.hardviz.com/v1/batches"),
      Some("http://localhost.example.com/v1/batches"),
      Some("http://192.168.1.10:8787/v1/batches"),
      Some("file:///tmp/batches"),
      Some("not a url"),
    ] {
      assert_eq!(debug_endpoint(refused), None, "{refused:?}");
    }
  }
}
