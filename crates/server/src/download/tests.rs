use super::*;
use axum::body::to_bytes;
use tower::ServiceExt;

#[test]
fn target_rejects_credentials_private_literals_and_ambiguous_syntax() {
    for value in [
        "http://example.org/a",
        "https://example.org:444/a",
        "https://user:secret@example.org/a",
        "https://example.org/a#fragment",
        "https://127.0.0.1/a",
        "https://2130706433/a",
        "https://0x7f000001/a",
        "https://[::1]/a",
        "https://example.org./a",
        "https://localhost/a",
        "https://example.org\\@localhost/a",
        "https://a..example.org/a",
    ] {
        assert!(target(value).is_err(), "accepted {value}");
    }
    assert!(target("https://downloads.example.org/a?signature=opaque").is_ok());
    assert_eq!(domain_input("EXAMPLE.ORG").as_deref(), Some("example.org"));
    for d in [
        "example.org/path",
        "example.org:443",
        "example.org?x=1",
        "*.example.org",
        "example.org.",
        "user@example.org",
    ] {
        assert!(domain_input(d).is_none(), "{d}");
    }
}
#[test]
fn address_filter_denies_special_ranges_in_both_families() {
    for ip in [
        "0.0.0.0",
        "10.1.2.3",
        "100.100.100.200",
        "127.0.0.1",
        "169.254.169.254",
        "168.63.129.16",
        "172.16.0.1",
        "192.168.1.1",
        "192.0.2.1",
        "198.18.0.1",
        "198.51.100.2",
        "203.0.113.2",
        "224.0.0.1",
        "255.255.255.255",
        "::",
        "::1",
        "::ffff:8.8.8.8",
        "64:ff9b::a00:1",
        "fc00::1",
        "fe80::1",
        "ff02::1",
        "2001:db8::1",
        "2002:a00:1::1",
        "2001::1",
        "3fff::1",
    ] {
        assert!(!public_ip(ip.parse().unwrap()), "{ip}");
    }
    for ip in [
        "1.1.1.1",
        "8.8.8.8",
        "2606:4700:4700::1111",
        "2001:4860:4860::8888",
    ] {
        assert!(public_ip(ip.parse().unwrap()), "{ip}");
    }
}
#[test]
fn ranges_are_single_bounded_or_open_ended_byte_intervals() {
    for r in ["bytes=0-9", "bytes=5-"] {
        let mut h = HeaderMap::new();
        h.insert(header::RANGE, r.parse().unwrap());
        assert!(range_header(&h).is_ok());
    }
    for r in [
        "bytes=-5",
        "bytes=9-1",
        "bytes=0-1,3-4",
        "items=0-1",
        "bytes=0-18446744073709551616",
    ] {
        let mut h = HeaderMap::new();
        h.insert(header::RANGE, r.parse().unwrap());
        assert!(range_header(&h).is_err());
    }
}
#[test]
fn runtime_bounds_concurrency_cancels_matching_domain_and_releases_permits() {
    let runtime = Arc::new(Runtime::default());
    let c = DownloadConfig {
        max_user_concurrent: 2,
        ..DownloadConfig::default()
    };
    let (one, mut cancelled) = runtime.acquire(1, &c).unwrap();
    let (two, _) = runtime.acquire(1, &c).unwrap();
    assert!(runtime.acquire(1, &c).is_none());
    runtime.host(&one.id, "cdn.example.org");
    runtime.cancel_domain("example.org", false);
    assert!(!*cancelled.borrow_and_update());
    runtime.cancel_domain("example.org", true);
    assert!(*cancelled.borrow_and_update());
    drop(two);
    assert!(runtime.acquire(1, &c).is_some());
    drop(one);
    assert_eq!(runtime.list(), json!([]));
}

#[tokio::test]
async fn download_routes_require_scoped_tokens_and_admin_browser_csrf() {
    let mut config = Config::default();
    config.download.enabled = true;
    let app = build_application(config).await.unwrap();
    let user = app
        .state
        .database
        .create_user("admin", "download@example.org", "Download", 12)
        .await
        .unwrap()
        .unwrap();
    let id = user.id;
    assert!(!app.state.database.download_allowed(id).await.unwrap());
    assert!(app
        .state
        .database
        .create_download_token(id, "test", 1)
        .await
        .unwrap()
        .is_none());
    let session = app
        .state
        .database
        .create_user_session(id, "test")
        .await
        .unwrap()
        .unwrap();
    let req = || Request::builder().uri("/api/download?url=https%3A%2F%2Fexample.org%2Fa");
    assert_eq!(
        app.router
            .clone()
            .oneshot(req().body(Body::empty()).unwrap())
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.router
            .clone()
            .oneshot(
                req()
                    .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    app.state
        .database
        .set_download_permission("admin", "user", id, "allow")
        .await
        .unwrap();
    let token = app
        .state
        .database
        .create_download_token(id, "test", 1)
        .await
        .unwrap()
        .unwrap();
    let token = token["token"].as_str().unwrap();
    // Credential authenticates only download endpoints, never administrator APIs.
    assert_eq!(
        app.router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/admin/api/download")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    app.state
        .database
        .set_download_block("admin", "example.org", true, "test", true)
        .await
        .unwrap();
    let r = app
        .router
        .clone()
        .oneshot(
            req()
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    assert!(
        String::from_utf8_lossy(&to_bytes(r.into_body(), 4096).await.unwrap()).contains("blocked")
    );
    let stats = app
        .state
        .database
        .download_stats(1, None, None)
        .await
        .unwrap();
    assert_eq!(stats["totals"][0]["denied"], 1);
    // Cookie token creation must obey the production CSRF middleware.
    let r = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/account/download/tokens")
                .header(
                    header::COOKIE,
                    format!("mirrorproxy_user_session_http={}", session.token),
                )
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"name":"test","days":1}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    app.state
        .database
        .set_download_permission("admin", "user", id, "deny")
        .await
        .unwrap();
    assert!(app
        .state
        .database
        .authenticate_download_token(token)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn pinned_transport_verifies_tls_hostname_and_does_not_follow_redirects() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let host = "artifacts.example.org";
    let generated = rcgen::generate_simple_self_signed(vec![host.into()]).unwrap();
    let cert = Certificate::from_pem(generated.cert.pem().as_bytes()).unwrap();
    let tls = axum_server::tls_rustls::RustlsConfig::from_pem(
        generated.cert.pem().into_bytes(),
        generated.signing_key.serialize_pem().into_bytes(),
    )
    .await
    .unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let captures = Arc::new(Mutex::new(Vec::<HeaderMap>::new()));
    let record = captures.clone();
    let app = Router::new()
        .route(
            "/artifact",
            get(move |h: HeaderMap| {
                record.lock().unwrap().push(h);
                async { ([(header::LOCATION, "https://127.0.0.1/private")], "abc") }
            }),
        )
        .route(
            "/redirect",
            get(|| async {
                (
                    StatusCode::FOUND,
                    [(header::LOCATION, "https://127.0.0.1/private")],
                )
            }),
        );
    let handle = axum_server::Handle::new();
    let h = handle.clone();
    tokio::spawn(async move {
        axum_server::from_tcp_rustls(listener, tls)
            .unwrap()
            .handle(h)
            .serve(app.into_make_service())
            .await
            .unwrap();
    });
    let url = format!("https://{host}:{}/artifact", address.port());
    // Loopback exists only in this controlled transport fixture. The public
    // target resolver rejects it before this builder in production.
    let strict = pinned_client(host, &[address]).build().unwrap();
    assert!(
        strict.get(&url).send().await.is_err(),
        "self-signed certificates must not be trusted"
    );
    let trusted = pinned_client(host, &[address])
        .add_root_certificate(cert)
        .build()
        .unwrap();
    assert_eq!(
        trusted
            .get(&url)
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap(),
        "abc"
    );
    let r = trusted
        .get(format!("https://{host}:{}/redirect", address.port()))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::FOUND);
    assert!(target(r.headers()[header::LOCATION].to_str().unwrap()).is_err());
    {
        let request = captures.lock().unwrap();
        assert_eq!(request.len(), 1);
        assert!(!request[0].contains_key(header::AUTHORIZATION));
        assert!(!request[0].contains_key(header::COOKIE));
    }
    assert!(trusted
        .get(format!("http://{host}:{}/artifact", address.port()))
        .send()
        .await
        .is_err());
    handle.shutdown();
}

/// Opt-in real upstream smoke; normal unit tests never depend on public services.
#[tokio::test]
#[ignore = "requires public HTTPS/DNS network access"]
async fn public_artifact_download_checks_payload_quota_stats_and_domain_block() {
    let mut config = Config::default();
    config.download.enabled = true;
    let app = build_application(config).await.unwrap();
    let user = app
        .state
        .database
        .create_user("admin", "smoke@example.org", "Smoke", 12)
        .await
        .unwrap()
        .unwrap();
    app.state
        .database
        .set_download_permission("admin", "user", user.id, "allow")
        .await
        .unwrap();
    let token = app
        .state
        .database
        .create_download_token(user.id, "Smoke", 1)
        .await
        .unwrap()
        .unwrap();
    let token = token["token"].as_str().unwrap().to_owned();
    let url = std::env::var("MIRRORPROXY_DOWNLOAD_SMOKE_URL")
        .unwrap_or_else(|_| "https://registry.npmjs.org/is-number/-/is-number-7.0.0.tgz".into());
    let host = Url::parse(&url).unwrap().host_str().unwrap().to_owned();
    let direct = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap()
        .get(&url)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = app.router.clone();
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(90))
        .build()
        .unwrap();
    let response = client
        .get(format!("http://{address}/api/download"))
        .bearer_auth(&token)
        .query(&[("url", &url)])
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "application/octet-stream"
    );
    assert!(response.headers()[header::CONTENT_DISPOSITION]
        .to_str()
        .unwrap()
        .starts_with("attachment"));
    let body = response.bytes().await.unwrap();
    assert_eq!(body, direct);
    assert!(!body.is_empty());
    let stats = app
        .state
        .database
        .download_stats(1, Some(user.id), None)
        .await
        .unwrap();
    assert_eq!(stats["totals"][0]["bytes"], body.len() as u64);
    assert_eq!(stats["totals"][0]["upstream_bytes"], body.len() as u64);
    app.state
        .database
        .set_download_block("admin", &host, true, "smoke", true)
        .await
        .unwrap();
    assert_eq!(
        client
            .get(format!("http://{address}/api/download"))
            .bearer_auth(&token)
            .query(&[("url", &url)])
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    println!("Verified public artifact: host={host}, bytes={}, SHA-256={:x}; statistics and domain block passed",body.len(),<Sha256 as sha2::Digest>::digest(&body));
    task.abort();
}

#[test]
fn download_config_defaults_validate_and_cli_keys_preserve_bounds() {
    let config = Config::default();
    assert!(!config.download.enabled);
    config.download.validate().unwrap();
    assert_eq!(
        plan_config_set(&config, "download.max_file_bytes", "4096")
            .unwrap()
            .toml_path,
        "download.max_file_bytes"
    );
    for (key, value) in [
        ("download.max_file_bytes", "0"),
        ("download.bandwidth_bytes_per_sec", "1"),
        ("download.max_concurrent", "0"),
        ("download.max_user_concurrent", "17"),
    ] {
        assert!(plan_config_set(&config, key, value).is_err());
    }
    assert_eq!(route_group_for_path("/api/download"), "/api/download");
}

#[test]
fn new_download_defaults_match_example_and_preserve_explicit_policies() {
    let defaults = DownloadConfig::default();
    assert!(!defaults.enabled);
    assert_eq!(defaults.max_file_bytes, 10 << 30);
    assert_eq!(defaults.daily_user_bytes, 20 << 30);
    assert_eq!(defaults.max_concurrent, 16);
    assert_eq!(defaults.max_user_concurrent, 5);
    assert_eq!(defaults.requests_per_minute, 300);
    assert_eq!(defaults.max_duration_secs, 1800);
    assert_eq!(defaults.bandwidth_bytes_per_sec, 100 << 20);
    defaults.validate().unwrap();
    let example: Config = toml::from_str(include_str!("../../../../config.example.toml")).unwrap();
    assert_eq!(example.download, defaults);
    let existing: DownloadConfig = serde_json::from_value(json!({
        "enabled": true, "max_file_bytes": 2_u64 << 30,
        "daily_user_bytes": 5_u64 << 30, "max_concurrent": 16,
        "max_user_concurrent": 2, "requests_per_minute": 30,
        "max_duration_secs": 1800, "bandwidth_bytes_per_sec": 20_u64 << 20
    }))
    .unwrap();
    assert!(existing.enabled);
    assert_eq!(existing.max_file_bytes, 2 << 30);
    assert_eq!(existing.max_user_concurrent, 2);
    assert_eq!(existing.requests_per_minute, 30);
    assert_eq!(existing.bandwidth_bytes_per_sec, 20 << 20);
    existing.validate().unwrap();
}
