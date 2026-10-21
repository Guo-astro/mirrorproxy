//! Public Hub HTTP downloads. No credentials, Xet transport, or server cache.
use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    response::Response,
};
use futures_util::TryStreamExt;
use reqwest::Url;

use super::ProxyError;
use crate::{config::Config, AppState};

// Explicit download hosts, never an arbitrary URL supplied by a client. Update
// alongside HF's documented storage endpoints, not from untrusted response data.
const DOWNLOAD_HOSTS: &[&str] = &[
    "huggingface.co",
    "cdn-lfs.huggingface.co",
    "cdn-lfs.hf.co",
    "cdn-lfs-us-1.hf.co",
    "cdn-lfs-eu-1.hf.co",
    "cas-bridge.xethub.hf.co",
    "cas-bridge.xethub-eu.hf.co",
    "us.aws.cdn.hf.co",
    "us.gcp.cdn.hf.co",
    "eu.aws.cdn.hf.co",
];
const METADATA: &[&str] = &["x-repo-commit", "x-linked-etag", "x-linked-size"];

pub async fn root(State(state): State<AppState>) -> Result<Response, ProxyError> {
    if !state.config().is_enabled("huggingface") {
        return Err(ProxyError::Disabled("huggingface"));
    }
    Response::builder()
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(
            "MirrorProxy public Hugging Face HTTP downloads (no server cache)\nSet HF_ENDPOINT to this URL and HF_HUB_DISABLE_XET=1.\n",
        ))
        .map_err(|_| ProxyError::InvalidHeader)
}

pub async fn proxy(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Result<Response, ProxyError> {
    let config = state.config();
    if !config.is_enabled("huggingface") {
        return Err(ProxyError::Disabled("huggingface"));
    }
    if !matches!(*request.method(), Method::GET | Method::HEAD) {
        return Err(ProxyError::MethodNotAllowed);
    }
    // Use the encoded URI, not Axum's decoded Path: branch names may contain
    // encoded slashes. Validate decoded segments before constructing the URL.
    let path = request
        .uri()
        .path()
        .strip_prefix("/huggingface/")
        .ok_or(ProxyError::InvalidUrl)?;
    let url = repository_url(&config.upstreams.huggingface, path, request.uri().query())?;
    let base_url = state.public_base_url(request.headers());
    let candidates = state.upstream_selector.rank(
        config.upstream_candidates_for(&url),
        &config.upstream_selection,
    );
    for (index, candidate) in candidates.iter().enumerate() {
        let started = std::time::Instant::now();
        let result = forward(
            &config,
            request.method().clone(),
            candidate.clone(),
            request.headers(),
            &base_url,
        )
        .await;
        match result {
            Ok(response) if !super::should_failover_status(response.status()) => {
                state
                    .upstream_selector
                    .record_success(candidate, started.elapsed());
                return Ok(response);
            }
            result => {
                state
                    .upstream_selector
                    .record_failure(candidate, &config.upstream_selection);
                if index + 1 == candidates.len() {
                    return result;
                }
            }
        }
    }
    unreachable!("an upstream group has at least one endpoint")
}

fn repository_url(base: &str, path: &str, query: Option<&str>) -> Result<Url, ProxyError> {
    validate_path(path)?;
    let mut url = Url::parse(super::select_upstream(base)?).map_err(|_| ProxyError::InvalidUrl)?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err(ProxyError::InvalidUrl);
    }
    url.set_path(&format!("{}/{path}", url.path().trim_end_matches('/')));
    url.set_query(query);
    if url.query_pairs().any(|(key, _)| {
        !matches!(
            key.as_ref(),
            "download"
                | "recursive"
                | "expand"
                | "expand[]"
                | "limit"
                | "cursor"
                | "blobs"
                | "files_metadata"
                | "securityStatus"
                | "full"
        )
    }) {
        return Err(ProxyError::InvalidUrl);
    }
    Ok(url)
}

fn validate_path(path: &str) -> Result<(), ProxyError> {
    let decoded = decode_path(path)?;
    if decoded.contains(['\\', '%', '?', '#'])
        || decoded.chars().any(char::is_control)
        || decoded
            .split('/')
            .any(|s| s.is_empty() || matches!(s, "." | ".."))
    {
        return Err(ProxyError::InvalidUrl);
    }
    let parts: Vec<_> = path.split('/').collect();
    let parts = parts.as_slice();
    let api = match parts {
        ["api", "models" | "datasets", rest @ ..] => {
            // Repository metadata and read-only tree listings used by snapshot_download.
            matches!(rest, [_] | [_, _])
                || matches!(rest, [_, "revision", _] | [_, _, "revision", _])
                || matches!(rest, [_, "tree", _, ..] | [_, _, "tree", _, ..])
        }
        ["api", "resolve-cache", "models" | "datasets", _, _, _, ..] => true,
        _ => false,
    };
    let file_parts = if parts.first() == Some(&"datasets") {
        &parts[1..]
    } else {
        parts
    };
    let file = matches!(
        file_parts,
        [_, "resolve", _, _, ..] | [_, _, "resolve", _, _, ..]
    );
    if api || file {
        Ok(())
    } else {
        Err(ProxyError::UnsupportedTarget)
    }
}

fn decode_path(path: &str) -> Result<String, ProxyError> {
    let mut bytes = Vec::with_capacity(path.len());
    let mut input = path.bytes();
    while let Some(byte) = input.next() {
        bytes.push(if byte == b'%' {
            let hi = input.next().and_then(|b| (b as char).to_digit(16));
            let lo = input.next().and_then(|b| (b as char).to_digit(16));
            match (hi, lo) {
                (Some(hi), Some(lo)) => (hi * 16 + lo) as u8,
                _ => return Err(ProxyError::InvalidUrl),
            }
        } else {
            byte
        });
    }
    String::from_utf8(bytes).map_err(|_| ProxyError::InvalidUrl)
}

fn same_origin(a: &Url, b: &Url) -> bool {
    a.origin() == b.origin()
}

pub(crate) async fn probe_endpoint(
    config: &Config,
    url: Url,
) -> Result<reqwest::Response, ProxyError> {
    crate::build_streaming_upstream_client_builder(config)
        .map_err(|_| ProxyError::InvalidUrl)?
        .build()?
        .get(url)
        .header(header::ACCEPT_ENCODING, "identity")
        .send()
        .await
        .map_err(|error| ProxyError::Upstream(error.without_url()))
}

fn allowed_redirect(original: &Url, previous: &Url, target: &Url) -> bool {
    matches!(target.scheme(), "http" | "https")
        && !(previous.scheme() == "https" && target.scheme() != "https")
        && target.username().is_empty()
        && target.password().is_none()
        && target.fragment().is_none()
        && (same_origin(original, target)
            || (target.scheme() == "https"
                && target.port_or_known_default() == Some(443)
                && target
                    .host_str()
                    .is_some_and(|h| DOWNLOAD_HOSTS.contains(&h))))
}

async fn forward(
    config: &Config,
    method: Method,
    original: Url,
    incoming: &HeaderMap,
    public_base: &str,
) -> Result<Response, ProxyError> {
    if !original.username().is_empty()
        || original.password().is_some()
        || original.fragment().is_some()
    {
        return Err(ProxyError::InvalidUrl);
    }
    let mut client = crate::build_streaming_upstream_client_builder(config)
        .map_err(|_| ProxyError::InvalidUrl)?
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .build()?;
    let mut url = original.clone();
    let mut metadata = HeaderMap::new();
    for hop in 0..=10 {
        // Deliberately bypass generic forwarding: neither client Authorization
        // nor configured upstream_auth may turn this public proxy into access
        // to a private/gated repository. Never consult the disk cache.
        let mut request = client
            .request(method.clone(), url.clone())
            .header(header::ACCEPT_ENCODING, "identity");
        for name in [
            "accept",
            "range",
            "if-range",
            "if-none-match",
            "if-modified-since",
        ] {
            if let Some(value) = incoming.get(name) {
                request = request.header(name, value);
            }
        }
        let response = request
            .send()
            .await
            .map_err(|error| ProxyError::Upstream(error.without_url()))?;
        if same_origin(&original, &url) {
            for &name in METADATA {
                if let Some(value) = response.headers().get(name) {
                    metadata.insert(name, value.clone());
                }
            }
        }
        if !matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
            let status = response.status();
            let mut headers = response.headers().clone();
            if status.is_success() {
                for (name, value) in &metadata {
                    headers.insert(name.clone(), value.clone());
                }
            }
            rewrite_pagination(&mut headers, &original, public_base)?;
            let body = if method == Method::HEAD {
                Body::empty()
            } else {
                Body::from_stream(
                    response
                        .bytes_stream()
                        .map_err(|error| std::io::Error::other(error.without_url())),
                )
            };
            return download_response(status, &headers, body);
        }
        let location = response
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(ProxyError::UnsafeRedirect)?;
        let target = url.join(location).map_err(|_| ProxyError::UnsafeRedirect)?;
        if hop == 10 || !allowed_redirect(&original, &url, &target) {
            return Err(ProxyError::UnsafeRedirect);
        }
        let explicit_ip_origin = same_origin(&original, &target)
            && original
                .host_str()
                .is_some_and(|h| h.parse::<std::net::IpAddr>().is_ok());
        let mut builder = crate::build_streaming_upstream_client_builder(config)
            .map_err(|_| ProxyError::UnsafeRedirect)?
            .no_gzip()
            .no_brotli()
            .no_deflate();
        if !explicit_ip_origin {
            // A remote outbound proxy may resolve DNS itself. Preserve the
            // existing fail-closed policy instead of bypassing DNS pinning.
            if config.outbound_proxy.enabled {
                return Err(ProxyError::UnsafeRedirect);
            }
            let addresses = super::oci::public_destination_addresses(&target)
                .await
                .map_err(|_| ProxyError::UnsafeRedirect)?;
            builder = builder.resolve_to_addrs(
                target.host_str().ok_or(ProxyError::UnsafeRedirect)?,
                &addresses,
            );
        }
        if method == Method::HEAD && !same_origin(&original, &target) {
            if let Some(response) = linked_head_response(&metadata)? {
                return Ok(response);
            }
        }
        client = builder.build()?;
        url = target;
    }
    unreachable!("redirect limit returns above")
}

fn linked_head_response(metadata: &HeaderMap) -> Result<Option<Response>, ProxyError> {
    if !metadata.contains_key("x-repo-commit")
        || !metadata.contains_key("x-linked-etag")
        || metadata
            .get("x-linked-size")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .is_none()
    {
        return Ok(None);
    }
    // HF metadata lives on the Hub's redirect, not on the CDN. Expose a normal
    // HTTP HEAD so the SDK's subsequent GET returns to this same proxy URL.
    let mut headers = metadata.clone();
    headers.insert(header::ETAG, headers["x-linked-etag"].clone());
    headers.insert(header::CONTENT_LENGTH, headers["x-linked-size"].clone());
    download_response(StatusCode::OK, &headers, Body::empty()).map(Some)
}

fn download_response(
    status: StatusCode,
    headers: &HeaderMap,
    body: Body,
) -> Result<Response, ProxyError> {
    let mut builder = Response::builder()
        .status(status)
        .header("x-mirrorproxy-cache", "BYPASS");
    for (name, value) in headers {
        if super::should_forward_response_header(name)
            && !name.as_str().starts_with("x-xet-")
            && !matches!(
                name.as_str(),
                "location" | "set-cookie" | "x-mirrorproxy-cache"
            )
        {
            builder = builder.header(name, value);
        }
    }
    builder.body(body).map_err(|_| ProxyError::InvalidHeader)
}

fn rewrite_pagination(
    headers: &mut HeaderMap,
    original: &Url,
    public_base: &str,
) -> Result<(), ProxyError> {
    let Some(value) = headers.remove(header::LINK) else {
        return Ok(());
    };
    // File Link headers advertise Xet endpoints; these must not escape to SDKs.
    if !original.path().contains("/api/") {
        return Ok(());
    }
    let base_path = original
        .path()
        .find("/api/")
        .ok_or(ProxyError::InvalidUrl)?;
    let mut links = Vec::new();
    for link in value
        .to_str()
        .map_err(|_| ProxyError::InvalidHeader)?
        .split(',')
    {
        let Some((target, relation)) = link
            .trim()
            .strip_prefix('<')
            .and_then(|s| s.split_once('>'))
        else {
            return Err(ProxyError::InvalidHeader);
        };
        if !relation.trim().starts_with("; rel=\"next\"") {
            continue;
        }
        let target = original
            .join(target)
            .map_err(|_| ProxyError::UnsafeRedirect)?;
        if !same_origin(original, &target) {
            return Err(ProxyError::UnsafeRedirect);
        }
        let path = target
            .path()
            .strip_prefix(&original.path()[..base_path])
            .ok_or(ProxyError::UnsafeRedirect)?;
        repository_url(
            "https://huggingface.co",
            path.trim_start_matches('/'),
            target.query(),
        )?;
        links.push(format!(
            "<{}/huggingface{}{}>; rel=\"next\"",
            public_base.trim_end_matches('/'),
            path,
            target.query().map(|q| format!("?{q}")).unwrap_or_default()
        ));
    }
    if !links.is_empty() {
        headers.insert(
            header::LINK,
            HeaderValue::from_str(&links.join(", ")).map_err(|_| ProxyError::InvalidHeader)?,
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn head_describes_the_linked_file_without_exposing_a_cdn_or_xet_endpoint() {
        let mut headers = HeaderMap::new();
        headers.insert("x-repo-commit", HeaderValue::from_static("commit"));
        headers.insert("x-linked-etag", HeaderValue::from_static("\"sha256\""));
        headers.insert("x-linked-size", HeaderValue::from_static("548105171"));
        let response = linked_head_response(&headers).unwrap().unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::ETAG], "\"sha256\"");
        assert_eq!(response.headers()[header::CONTENT_LENGTH], "548105171");
        assert_eq!(response.headers()["x-repo-commit"], "commit");
        assert!(!response.headers().contains_key(header::LOCATION));
        assert!(axum::body::to_bytes(response.into_body(), 1)
            .await
            .unwrap()
            .is_empty());
        headers.insert("x-linked-size", HeaderValue::from_static("invalid"));
        assert!(linked_head_response(&headers).unwrap().is_none());
        headers.remove("x-repo-commit");
        assert!(linked_head_response(&headers).unwrap().is_none());
    }

    #[test]
    fn accepts_only_download_and_repository_metadata_paths() {
        for path in [
            "gpt2/resolve/main/config.json",
            "org/model/resolve/refs%2Fpr%2F1/dir/model.safetensors",
            "datasets/org/data/resolve/main/data/part.parquet",
            "api/models/gpt2/revision/main",
            "api/datasets/org/data/tree/main/data",
            "api/models/org/model/revision/refs%2Fpr%2F1",
            "api/resolve-cache/models/gpt2/abcdef/config.json",
        ] {
            assert!(
                repository_url("https://huggingface.co", path, None).is_ok(),
                "{path}"
            );
        }
        for path in [
            "",
            "../config.json",
            "gpt2/resolve/main/%2e%2e/config.json",
            "gpt2/resolve/main/%252e%252e/config.json",
            "gpt2/resolve/main/a%2f..%2fb",
            "gpt2/resolve/main/a\\b",
            "gpt2/resolve/main/a%00b",
            "gpt2//resolve/main/a",
            "https://example.com/file",
            "api/whoami-v2",
            "api/models",
            "api/models/gpt2/xet-read-token/main",
            "org/model/raw/main/file",
            "org/model.git/info/refs",
            "org/model/resolve/main/a%zz",
        ] {
            assert!(
                repository_url("https://huggingface.co", path, None).is_err(),
                "{path}"
            );
        }
        assert!(repository_url(
            "https://huggingface.co",
            "gpt2/resolve/main/a",
            Some("token=secret")
        )
        .is_err());
        assert_eq!(
            repository_url(
                "https://hf.example/hub",
                "gpt2/resolve/main/config.json",
                Some("download=true")
            )
            .unwrap()
            .as_str(),
            "https://hf.example/hub/gpt2/resolve/main/config.json?download=true"
        );
    }

    #[test]
    fn redirects_restrict_hosts_schemes_ports_and_credentials() {
        let original = Url::parse("https://huggingface.co/gpt2/resolve/main/a").unwrap();
        for target in [
            "https://cdn-lfs-us-1.hf.co/a?Signature=opaque",
            "https://us.aws.cdn.hf.co/a",
            "https://huggingface.co/api/resolve-cache/models/gpt2/abc/a",
        ] {
            assert!(allowed_redirect(
                &original,
                &original,
                &Url::parse(target).unwrap()
            ));
        }
        for target in [
            "https://huggingface.co.evil.example/a",
            "https://evil.hf.co/a",
            "http://huggingface.co/a",
            "https://us.aws.cdn.hf.co:444/a",
            "https://user:secret@us.aws.cdn.hf.co/a",
            "https://127.0.0.1/a",
            "https://169.254.169.254/latest/meta-data",
            "https://us.aws.cdn.hf.co/a#fragment",
        ] {
            assert!(
                !allowed_redirect(&original, &original, &Url::parse(target).unwrap()),
                "{target}"
            );
        }
    }

    #[test]
    fn pagination_stays_on_the_public_proxy_and_xet_links_are_removed() {
        let original = Url::parse("https://hf.example/hub/api/models/org/model/tree/main").unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(header::LINK, HeaderValue::from_static("<https://hf.example/hub/api/models/org/model/tree/main?cursor=abc%3D&limit=1>; rel=\"next\""));
        rewrite_pagination(&mut headers, &original, "https://user.mirror.example").unwrap();
        assert_eq!(headers[header::LINK], "<https://user.mirror.example/huggingface/api/models/org/model/tree/main?cursor=abc%3D&limit=1>; rel=\"next\"");
        headers.insert(
            header::LINK,
            HeaderValue::from_static("<https://evil.example/api/models/gpt2>; rel=\"next\""),
        );
        assert!(rewrite_pagination(&mut headers, &original, "https://mirror.example").is_err());
        headers.insert(
            header::LINK,
            HeaderValue::from_static(
                "<https://cas-server.xethub.hf.co>; rel=\"xet-reconstruction-info\"",
            ),
        );
        headers.insert("x-xet-hash", HeaderValue::from_static("hash"));
        rewrite_pagination(
            &mut headers,
            &Url::parse("https://huggingface.co/gpt2/resolve/main/a").unwrap(),
            "https://mirror.example",
        )
        .unwrap();
        let response = download_response(StatusCode::OK, &headers, Body::empty()).unwrap();
        assert!(!response.headers().contains_key(header::LINK));
        assert!(!response.headers().contains_key("x-xet-hash"));
        assert_eq!(response.headers()["x-mirrorproxy-cache"], "BYPASS");
    }
}
