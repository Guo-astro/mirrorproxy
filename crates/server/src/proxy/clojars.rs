use super::ProxyError;
use super::{repository_url, sanitize_repository_path as sanitize};
use crate::{proxy, AppState};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderValue},
    response::Response,
};

pub async fn root(State(state): State<AppState>) -> Result<Response, ProxyError> {
    if !state.config().is_enabled("clojars") {
        return Err(ProxyError::Disabled("clojars"));
    }
    Response::builder()
        .status(200)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        )
        .body(Body::from("MirrorProxy Clojars repository proxy\n"))
        .map_err(|_| ProxyError::InvalidHeader)
}
pub async fn proxy(
    State(state): State<AppState>,
    Path(path): Path<String>,
    request: axum::extract::Request,
) -> Result<Response, ProxyError> {
    let config = state.config();
    if !config.is_enabled("clojars") {
        return Err(ProxyError::Disabled("clojars"));
    }
    let path = sanitize(&path)?;
    let url = repository_url(&config.upstreams.clojars, path, request.uri().query())?;
    proxy::forward(&state, request.method().clone(), url, request.headers()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_maven_paths() {
        assert!(sanitize("org/clojure/clojure/1.12.0/clojure-1.12.0.pom").is_ok());
    }
    #[test]
    fn rejects_unsafe_paths() {
        assert!(sanitize("../deps.edn").is_err());
    }
    #[test]
    fn keeps_base_path() {
        assert_eq!(
            repository_url(
                "https://mirror.example/clojars",
                "foo/bar/maven-metadata.xml",
                None
            )
            .unwrap()
            .as_str(),
            "https://mirror.example/clojars/foo/bar/maven-metadata.xml"
        );
    }
}
