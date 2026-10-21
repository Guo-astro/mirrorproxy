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
    if !state.config().is_enabled("anaconda") {
        return Err(ProxyError::Disabled("anaconda"));
    }
    Response::builder()
        .status(200)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        )
        .body(Body::from("MirrorProxy Anaconda repository proxy\n"))
        .map_err(|_| ProxyError::InvalidHeader)
}
pub async fn proxy(
    State(state): State<AppState>,
    Path(path): Path<String>,
    request: axum::extract::Request,
) -> Result<Response, ProxyError> {
    let config = state.config();
    if !config.is_enabled("anaconda") {
        return Err(ProxyError::Disabled("anaconda"));
    }
    let path = sanitize(&path)?;
    let url = repository_url(&config.upstreams.anaconda, path, request.uri().query())?;
    proxy::forward(&state, request.method().clone(), url, request.headers()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_conda_paths() {
        assert!(sanitize("main/linux-64/repodata.json").is_ok());
        assert!(sanitize("main/linux-64/python-3.12.0.conda").is_ok());
    }
    #[test]
    fn rejects_unsafe() {
        assert!(sanitize("../.condarc").is_err());
    }
}
