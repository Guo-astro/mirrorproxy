//! Password accounts and administrator-issued registration links, independent of SMTP.
use crate::{
    database::local_accounts::{AccountBilling, LocalCreateOutcome, LocalUserCreation},
    *,
};
use axum::{
    extract::{ConnectInfo, Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde::Deserialize;
use std::{net::SocketAddr, time::Instant};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Credentials {
    username: String,
    password: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewAccount {
    username: String,
    password: String,
    display_name: String,
    #[serde(default)]
    billing: AccountBilling,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Registration {
    token: String,
    username: String,
    password: String,
    #[serde(default)]
    display_name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewInvitation {
    billing: AccountBilling,
    expires_in_hours: u32,
}

fn validate_account(username: &str, password: &str, display_name: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        (3..=64).contains(&username.len())
            && username
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.')),
        "account must contain 3 to 64 letters, digits, dots, underscores or hyphens"
    );
    anyhow::ensure!(
        !display_name.is_empty() && display_name.chars().count() <= 100,
        "display_name must contain 1 to 100 characters"
    );
    anyhow::ensure!(password.len() <= 1024, "password is too long");
    anyhow::ensure!(!password.is_empty(), "password must not be empty");
    Ok(())
}

pub(crate) async fn create(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<NewAccount>,
) -> Response {
    let actor = match require_super_admin(&headers, &state).await {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    if let Err(error) = validate_account(
        request.username.trim(),
        &request.password,
        request.display_name.trim(),
    )
    .and_then(|()| request.billing.validate())
    {
        return bad_request_response(error.to_string());
    }
    let result = state
        .database
        .create_local_user(LocalUserCreation {
            actor: &actor.username,
            username: request.username.trim(),
            password: request.password,
            display_name: request.display_name.trim(),
            billing: &request.billing,
            invitation_token: None,
            routing_length: state.config().user_access.routing_id_min_length,
        })
        .await;
    creation_response(&state, result, false).await
}

pub(crate) async fn invite(
    headers: HeaderMap,
    State(state): State<AppState>,
    native_https: Option<Extension<HttpsTransport>>,
    Json(request): Json<NewInvitation>,
) -> Response {
    let actor = match require_super_admin(&headers, &state).await {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    if state.config().registration.mode == "disabled" {
        return bad_request_response("new registration is disabled".into());
    }
    if !(1..=720).contains(&request.expires_in_hours) {
        return bad_request_response("invitation lifetime must be 1 to 720 hours".into());
    }
    if let Err(error) = request.billing.validate() {
        return bad_request_response(error.to_string());
    }
    // Forwarded headers have already been stripped unless the immediate peer is
    // trusted. Reuse the validated request authority when no public URL is set.
    let config = state.config();
    let base = state.public_base_url(&headers);
    let mut url = match Url::parse(&base) {
        Ok(url)
            if matches!(url.scheme(), "http" | "https")
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none() =>
        {
            url
        }
        _ => {
            return bad_request_response(
                "cannot determine a valid public URL for the invitation".into(),
            )
        }
    };
    if config.public_base_url.is_empty() && native_https.is_some() {
        let _ = url.set_scheme("https");
    }
    url.set_path("/login");
    url.set_query(None);
    match state
        .database
        .create_registration_invitation(&actor.username, &request.billing, request.expires_in_hours)
        .await
    {
        Ok(Some((id, token))) => {
            // Fragments stay out of HTTP access logs and Referer headers.
            url.set_fragment(Some(&format!("register={token}")));
            let mut response = (
                StatusCode::CREATED,
                Json(serde_json::json!({ "id": id, "url": url.as_str() })),
            )
                .into_response();
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
        }
        Ok(None) => bad_request_response("billing group not found".into()),
        Err(error) => {
            tracing::error!(%error, "failed to create registration invitation");
            internal_error_response()
        }
    }
}

pub(crate) async fn list(headers: HeaderMap, State(state): State<AppState>) -> Response {
    if let Err(response) = require_super_admin(&headers, &state).await {
        return response;
    }
    match state.database.registration_invitations().await {
        Ok(items) => Json(items).into_response(),
        Err(error) => {
            tracing::error!(%error, "failed to list registration invitations");
            internal_error_response()
        }
    }
}

pub(crate) async fn revoke(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    let actor = match require_super_admin(&headers, &state).await {
        Ok(actor) => actor,
        Err(response) => return response,
    };
    match state
        .database
        .revoke_registration_invitation(&actor.username, id)
        .await
    {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!(%error, "failed to revoke registration invitation");
            internal_error_response()
        }
    }
}

fn allow_attempt(state: &AppState, headers: &HeaderMap, peer: SocketAddr, key: &str) -> bool {
    // Require same-origin for anonymous credential submissions, including login CSRF.
    if !request_has_same_origin(headers) {
        return false;
    }
    let source = resolve_client_ip_from(peer.ip(), headers, &state.config());
    let key = format!("local-{key}:{source}");
    let now = Instant::now();
    if state.admin_login_limiter.is_limited(&key, 20, now) {
        return false;
    }
    state.admin_login_limiter.record(&key, now);
    true
}

pub(crate) async fn register(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(request): Json<Registration>,
) -> Response {
    if !allow_attempt(&state, &headers, peer, "register") {
        return StatusCode::FORBIDDEN.into_response();
    }
    if state.config().registration.mode == "disabled" {
        return StatusCode::FORBIDDEN.into_response();
    }
    if request.token.len() != 48 {
        return bad_request_response("invalid invitation".into());
    }
    let username = request.username.trim();
    let display_name = match request.display_name.trim() {
        "" => username,
        name => name,
    };
    if let Err(error) = validate_account(username, &request.password, display_name) {
        return bad_request_response(error.to_string());
    }
    let result = state
        .database
        .create_local_user(LocalUserCreation {
            actor: "invitation",
            username,
            password: request.password,
            display_name,
            billing: &AccountBilling::default(),
            invitation_token: Some(&request.token),
            routing_length: state.config().user_access.routing_id_min_length,
        })
        .await;
    creation_response(&state, result, true).await
}

async fn creation_response(
    state: &AppState,
    result: anyhow::Result<LocalCreateOutcome>,
    sign_in: bool,
) -> Response {
    match result {
        Ok(LocalCreateOutcome::Created(id)) if sign_in => {
            match state.database.create_user_session(id, "password").await {
                Ok(Some(session)) => session_response(session),
                _ => internal_error_response(),
            }
        }
        Ok(LocalCreateOutcome::Created(id)) => match state.database.user_account(id).await {
            Ok(Some(user)) => (StatusCode::CREATED, Json(user)).into_response(),
            _ => internal_error_response(),
        },
        Ok(LocalCreateOutcome::UsernameExists) => conflict_response("account already exists"),
        Ok(LocalCreateOutcome::InvalidInvitation) => {
            bad_request_response("invitation is invalid, expired or already used".into())
        }
        Ok(LocalCreateOutcome::InvalidGroup) => {
            bad_request_response("billing group not found".into())
        }
        Err(error) => {
            tracing::error!(%error, "failed to create local account");
            internal_error_response()
        }
    }
}

pub(crate) async fn login(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(request): Json<Credentials>,
) -> Response {
    if !request_has_same_origin(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    if !allow_attempt(&state, &headers, peer, "login") {
        return too_many_login_attempts_response(900);
    }
    if request.username.len() > 64 || request.password.len() > 1024 {
        return unauthorized_response();
    }
    match state
        .database
        .authenticate_local_user(request.username.trim(), request.password)
        .await
    {
        Ok(Some(session)) => session_response(session),
        Ok(None) => unauthorized_response(),
        Err(error) => {
            tracing::error!(%error, "local password login failed");
            internal_error_response()
        }
    }
}

fn session_response(session: database::UserSession) -> Response {
    let mut response =
        Json(serde_json::json!({ "user_id": session.identity.user_id })).into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, user_session_cookie(&session.token));
    response
        .headers_mut()
        .append(header::SET_COOKIE, user_csrf_cookie(&session.token));
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PasswordChange {
    current_password: String,
    new_password: String,
}

pub(crate) async fn change_password(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<PasswordChange>,
) -> Response {
    let identity = match authenticated_user(&headers, &state).await {
        Ok(Some(identity)) => identity,
        Ok(None) => return unauthorized_response(),
        Err(_) => return internal_error_response(),
    };
    let attempt_key = format!("local-password-change:{}", identity.user_id);
    let now = Instant::now();
    if state.admin_login_limiter.is_limited(&attempt_key, 5, now) {
        return too_many_login_attempts_response(900);
    }
    state.admin_login_limiter.record(&attempt_key, now);
    let username = match state.database.user_account(identity.user_id).await {
        Ok(Some(user)) => user.username.unwrap_or_default(),
        _ => return unauthorized_response(),
    };
    if let Err(error) = validate_account(&username, &request.new_password, &identity.display_name) {
        return bad_request_response(error.to_string());
    }
    if request.current_password.len() > 1024 {
        return unauthorized_response();
    }
    match state
        .database
        .change_local_password(
            identity.user_id,
            request.current_password,
            request.new_password,
        )
        .await
    {
        Ok(true) => {
            let mut response = StatusCode::NO_CONTENT.into_response();
            response
                .headers_mut()
                .insert(header::SET_COOKIE, clear_user_session_cookie());
            response
                .headers_mut()
                .append(header::SET_COOKIE, clear_user_csrf_cookie());
            response
        }
        Ok(false) => unauthorized_response(),
        Err(error) => {
            tracing::error!(%error, "failed to change user password");
            internal_error_response()
        }
    }
}

#[cfg(test)]
mod password_policy_tests {
    use super::*;

    #[test]
    fn local_passwords_only_require_nonempty_content_with_bounded_size() {
        let password = uuid::Uuid::new_v4().simple().to_string();
        let password = password.repeat(33);
        for length in [1, 3, 5, 12, 1024] {
            assert!(validate_account("friend", &password[..length], "Friend").is_ok());
        }
        let whitespace = loop {
            let candidate = char::from(rand::random::<u8>());
            if candidate.is_whitespace() {
                break candidate.to_string();
            }
        };
        assert!(validate_account("friend", &whitespace, "Friend").is_ok());
        let empty_password = String::new();
        assert!(validate_account("friend", &empty_password, "Friend").is_err());
        assert!(validate_account("friend", &password[..1025], "Friend").is_err());
    }
}
