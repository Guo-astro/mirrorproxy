use super::{repository_url, sanitize_repository_path};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderValue},
    response::Response,
};

use super::ProxyError;
use crate::{proxy, AppState};

pub async fn root(State(state): State<AppState>) -> Result<Response, ProxyError> {
    if !state.config().is_enabled("hackage") {
        return Err(ProxyError::Disabled("hackage"));
    }
    Response::builder()
        .status(200)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        )
        .body(Body::from("MirrorProxy Hackage repository proxy\n"))
        .map_err(|_| ProxyError::InvalidHeader)
}

pub async fn proxy(
    State(state): State<AppState>,
    Path(path): Path<String>,
    request: axum::extract::Request,
) -> Result<Response, ProxyError> {
    let config = state.config();
    if !config.is_enabled("hackage") {
        return Err(ProxyError::Disabled("hackage"));
    }
    let path = sanitize_repository_path(&path)?;
    let url = repository_url(&config.upstreams.hackage, path, request.uri().query())?;
    proxy::forward(&state, request.method().clone(), url, request.headers()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_index_and_package_paths() {
        assert!(sanitize_repository_path("01-index.tar.gz").is_ok());
        assert!(sanitize_repository_path("package/foo-1.0/foo-1.0.tar.gz").is_ok());
    }
    #[test]
    fn rejects_unsafe_paths() {
        assert!(sanitize_repository_path("../cabal/config").is_err());
        assert!(sanitize_repository_path("package//foo").is_err());
    }
    #[test]
    fn preserves_base_path() {
        assert_eq!(
            repository_url("https://mirror.example/hackage", "01-index.tar.gz", None)
                .unwrap()
                .as_str(),
            "https://mirror.example/hackage/01-index.tar.gz"
        );
    }
}
