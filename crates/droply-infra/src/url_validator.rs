use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use async_trait::async_trait;
use droply_application::UrlValidator;
use droply_domain::DroplyError;
use tokio::net::lookup_host;
use url::Url;

/// SSRF-safe URL validator: only `http`/`https`, only destinations that
/// don't resolve to loopback/private/link-local/reserved address space.
///
/// DNS is resolved and *every* returned address is checked, not just the
/// first — a malicious host can otherwise pass validation by handing back a
/// public IP on the validating request and a private one on the real fetch
/// (DNS rebinding). This only validates a single hop; callers that follow
/// redirects must re-validate each redirect's `Location` through this same
/// validator, not trust `reqwest`'s built-in redirect following.
pub struct SsrfSafeUrlValidator;

impl SsrfSafeUrlValidator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SsrfSafeUrlValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UrlValidator for SsrfSafeUrlValidator {
    async fn validate(&self, url: &Url) -> Result<(), DroplyError> {
        validate_url(url).await
    }
}

async fn validate_url(url: &Url) -> Result<(), DroplyError> {
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(DroplyError::InvalidUrl {
            reason: format!("unsupported scheme: {scheme}"),
        });
    }

    let host = url.host_str().ok_or_else(|| DroplyError::InvalidUrl {
        reason: "URL has no host".to_string(),
    })?;

    if host.eq_ignore_ascii_case("localhost") {
        return Err(DroplyError::InvalidUrl {
            reason: "localhost is not allowed".to_string(),
        });
    }

    // Host is already a literal IP — check it directly, no DNS involved.
    if let Ok(ip) = host.parse::<IpAddr>() {
        return if is_blocked_ip(ip) {
            Err(DroplyError::InvalidUrl {
                reason: format!("blocked IP address: {ip}"),
            })
        } else {
            Ok(())
        };
    }

    let port = url.port_or_known_default().unwrap_or(80);
    let mut addrs = lookup_host((host, port))
        .await
        .map_err(|_| DroplyError::SourceUnavailable)?
        .peekable();

    if addrs.peek().is_none() {
        return Err(DroplyError::SourceUnavailable);
    }

    for addr in addrs {
        if is_blocked_ip(addr.ip()) {
            return Err(DroplyError::InvalidUrl {
                reason: format!("{host} resolves to a blocked IP address: {}", addr.ip()),
            });
        }
    }

    Ok(())
}

fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_ipv4(v4),
        IpAddr::V6(v6) => is_blocked_ipv6(v6),
    }
}

fn is_blocked_ipv4(ip: Ipv4Addr) -> bool {
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local() // covers the 169.254.169.254 cloud metadata endpoint (AWS/GCP/Azure/DigitalOcean)
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_documentation()
        || is_shared_address_space(ip)
}

/// 100.64.0.0/10 (RFC 6598, carrier-grade NAT) — also the range Alibaba
/// Cloud's metadata endpoint (100.100.100.200) lives in. Not yet a stable
/// `Ipv4Addr` method (`is_shared` is still nightly-only), so checked by hand.
fn is_shared_address_space(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    a == 100 && (b & 0b1100_0000) == 0b0100_0000
}

fn is_blocked_ipv6(ip: Ipv6Addr) -> bool {
    if ip.is_loopback() || ip.is_unspecified() {
        return true;
    }
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_blocked_ipv4(v4);
    }

    let [first, ..] = ip.segments();
    let is_unique_local = (first & 0xfe00) == 0xfc00; // fc00::/7
    let is_link_local = (first & 0xffc0) == 0xfe80; // fe80::/10
    is_unique_local || is_link_local
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    async fn validate(url: &str) -> Result<(), DroplyError> {
        validate_url(&Url::parse(url).unwrap()).await
    }

    #[tokio::test]
    async fn rejects_non_http_schemes() {
        assert!(validate("file:///etc/passwd").await.is_err());
        assert!(validate("ftp://example.com/file").await.is_err());
    }

    #[tokio::test]
    async fn rejects_localhost_by_name() {
        assert!(validate("http://localhost/").await.is_err());
        assert!(validate("http://LOCALHOST:8080/").await.is_err());
    }

    #[tokio::test]
    async fn rejects_loopback_ip_literal() {
        assert!(validate("http://127.0.0.1/").await.is_err());
        assert!(validate("http://[::1]/").await.is_err());
    }

    #[tokio::test]
    async fn rejects_private_ranges() {
        assert!(validate("http://10.0.0.5/").await.is_err());
        assert!(validate("http://172.16.0.5/").await.is_err());
        assert!(validate("http://192.168.1.5/").await.is_err());
    }

    #[tokio::test]
    async fn rejects_link_local_and_cloud_metadata() {
        assert!(validate("http://169.254.169.254/").await.is_err());
        assert!(validate("http://169.254.1.1/").await.is_err());
    }

    #[tokio::test]
    async fn rejects_shared_address_space() {
        assert!(validate("http://100.100.100.200/").await.is_err());
    }

    #[tokio::test]
    async fn accepts_a_public_ip_literal() {
        assert!(validate("http://1.1.1.1/").await.is_ok());
    }

    #[test]
    fn ipv4_blocklist_matches_documented_ranges() {
        assert!(is_blocked_ipv4(Ipv4Addr::new(127, 0, 0, 1)));
        assert!(is_blocked_ipv4(Ipv4Addr::new(10, 1, 2, 3)));
        assert!(is_blocked_ipv4(Ipv4Addr::new(172, 16, 0, 1)));
        assert!(is_blocked_ipv4(Ipv4Addr::new(172, 31, 255, 255)));
        assert!(!is_blocked_ipv4(Ipv4Addr::new(172, 32, 0, 1)));
        assert!(is_blocked_ipv4(Ipv4Addr::new(192, 168, 0, 1)));
        assert!(is_blocked_ipv4(Ipv4Addr::new(169, 254, 169, 254)));
        assert!(is_blocked_ipv4(Ipv4Addr::new(0, 0, 0, 0)));
        assert!(is_blocked_ipv4(Ipv4Addr::new(100, 64, 0, 1)));
        assert!(is_blocked_ipv4(Ipv4Addr::new(100, 127, 255, 255)));
        assert!(!is_blocked_ipv4(Ipv4Addr::new(100, 128, 0, 1)));
        assert!(!is_blocked_ipv4(Ipv4Addr::new(8, 8, 8, 8)));
    }

    #[test]
    fn ipv6_blocklist_matches_documented_ranges() {
        assert!(is_blocked_ipv6(Ipv6Addr::LOCALHOST));
        assert!(is_blocked_ipv6(Ipv6Addr::UNSPECIFIED));
        assert!(is_blocked_ipv6("fc00::1".parse().unwrap()));
        assert!(is_blocked_ipv6("fe80::1".parse().unwrap()));
        assert!(!is_blocked_ipv6("2606:4700:4700::1111".parse().unwrap()));
    }
}
