use super::{repository_url, sanitize_repository_path};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderValue},
    response::Response,
};

use crate::{proxy, AppState};

use super::ProxyError;

/// Proxies CRAN's repository layout for source packages, platform binaries,
/// package indexes, and archive downloads without exposing arbitrary targets.
pub async fn root(State(state): State<AppState>) -> Result<Response, ProxyError> {
    if !state.config().is_enabled("cran") {
        return Err(ProxyError::Disabled("cran"));
    }

    Response::builder()
        .status(200)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        )
        .body(Body::from("MirrorProxy CRAN repository proxy\n"))
        .map_err(|_| ProxyError::InvalidHeader)
}

pub async fn proxy(
    State(state): State<AppState>,
    Path(path): Path<String>,
    request: axum::extract::Request,
) -> Result<Response, ProxyError> {
    let config = state.config();
    if !config.is_enabled("cran") {
        return Err(ProxyError::Disabled("cran"));
    }

    let clean_path = sanitize_repository_path(&path)?;
    let url = repository_url(&config.upstreams.cran, clean_path, request.uri().query())?;
    proxy::forward(&state, request.method().clone(), url, request.headers()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_cran_index_and_package_paths() {
        assert!(sanitize_repository_path("src/contrib/PACKAGES.gz").is_ok());
        assert!(
            sanitize_repository_path("src/contrib/Archive/digest/digest_0.6.37.tar.gz").is_ok()
        );
        assert!(
            sanitize_repository_path("bin/linux/ubuntu/jammy-cran40/x86_64/Packages.gz").is_ok()
        );
    }

    #[test]
    fn rejects_traversal_and_empty_paths() {
        assert!(sanitize_repository_path("../.Rprofile").is_err());
        assert!(sanitize_repository_path("src/contrib//PACKAGES").is_err());
        assert!(sanitize_repository_path("src\\contrib\\PACKAGES").is_err());
    }

    #[test]
    fn preserves_configured_mirror_base_path() {
        assert_eq!(
            repository_url(
                "https://mirror.example/cran",
                "src/contrib/PACKAGES",
                Some("type=source"),
            )
            .unwrap()
            .as_str(),
            "https://mirror.example/cran/src/contrib/PACKAGES?type=source"
        );
    }
}
