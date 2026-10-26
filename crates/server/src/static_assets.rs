use axum::{
    body::Body,
    http::{header, HeaderValue, StatusCode},
    response::Response,
};
use include_dir::{include_dir, Dir};

use crate::config::SiteConfig;

static WEB_DIST: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../web/dist");

pub fn serve(path: &str, site: &SiteConfig, canonical_base_url: &str) -> Response {
    // Serve installers from the same checkout as the UI, independent of main/cache.
    let installer = match path {
        "/install.sh" => Some(include_bytes!("../../../scripts/install.sh").as_slice()),
        "/install.ps1" => Some(include_bytes!("../../../scripts/install.ps1").as_slice()),
        _ => None,
    };
    if let Some(bytes) = installer {
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .header(header::CACHE_CONTROL, "no-cache")
            .header("x-content-type-options", "nosniff")
            .body(Body::from(bytes))
            .expect("installer response should be valid");
    }
    let normalized = normalize_path(path);
    let asset_path = if normalized.is_empty() {
        "index.html"
    } else {
        normalized.as_str()
    };

    if let Some(file) = WEB_DIST.get_file(asset_path) {
        return if asset_path == "index.html" {
            index_response(file.contents(), path, site, canonical_base_url)
        } else {
            asset_response(asset_path, file.contents())
        };
    }

    // Missing hashed chunks must not return SPA HTML with an immutable cache key.
    if [
        "js",
        "css",
        "map",
        "woff",
        "woff2",
        "ttf",
        "png",
        "svg",
        "ico",
        "webmanifest",
    ]
    .iter()
    .any(|extension| normalized.ends_with(&format!(".{extension}")))
    {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("not found"))
            .expect("static response should be valid");
    }

    if let Some(file) = WEB_DIST.get_file("index.html") {
        return index_response(file.contents(), path, site, canonical_base_url);
    }

    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("not found"))
        .expect("static response should be valid")
}

fn index_response(
    bytes: &'static [u8],
    request_path: &str,
    site: &SiteConfig,
    canonical_base_url: &str,
) -> Response {
    let mut html = String::from_utf8_lossy(bytes).into_owned();
    let title = escape_html(site.title.trim());
    let description = escape_html(site.description.trim());
    let keywords = escape_html(&site.keywords.join(", "));
    let icon = escape_html(site.icon_url.trim());
    let canonical = escape_html(canonical_base_url.trim_end_matches('/'));
    if let (Some(start), Some(end)) = (html.find("<title>"), html.find("</title>")) {
        if start < end {
            html.replace_range(
                start..end + "</title>".len(),
                &format!("<title>{title}</title>"),
            );
        }
    }
    for default_icon in [
        "<link rel=\"shortcut icon\" href=\"/favicon.svg\">",
        "<link rel=\"icon\" href=\"/favicon.svg\" type=\"image/svg+xml\" />",
        "<link rel=\"icon\" href=\"/favicon.svg\"/>",
        "<link rel=\"icon\" href=\"/favicon.svg\">",
        "<link rel=\"icon\" href=\"/favicon.svg\" />",
    ] {
        html = html.replace(
            default_icon,
            &format!("<link rel=\"icon\" href=\"{icon}\" />"),
        );
    }
    // Keep the high-resolution touch icon for the built-in brand. Custom site
    // icons still apply to both browser and touch icons, as before.
    let touch_icon = if site.icon_url.trim() == "/favicon.svg" {
        "/apple-touch-icon.png"
    } else {
        &icon
    };
    html = html.replace(
        "href=\"/apple-touch-icon.png\"",
        &format!("href=\"{touch_icon}\""),
    );
    let mut metadata = String::new();
    if !html.contains("rel=\"apple-touch-icon\"") {
        metadata.push_str(&format!(
            "    <link rel=\"apple-touch-icon\" href=\"{touch_icon}\" />\n"
        ));
    }
    if !description.is_empty() {
        metadata.push_str(&format!(
            "    <meta name=\"description\" content=\"{description}\" />\n    <meta property=\"og:description\" content=\"{description}\" />\n"
        ));
    }
    if !keywords.is_empty() {
        metadata.push_str(&format!(
            "    <meta name=\"keywords\" content=\"{keywords}\" />\n"
        ));
    }
    metadata.push_str(&format!(
        "    <meta property=\"og:title\" content=\"{title}\" />\n    <meta property=\"og:type\" content=\"website\" />\n"
    ));
    if !canonical.is_empty() {
        metadata.push_str(&format!(
            "    <link rel=\"canonical\" href=\"{canonical}\" />\n    <meta property=\"og:url\" content=\"{canonical}\" />\n"
        ));
    }
    if is_private_page(request_path) {
        metadata.push_str("    <meta name=\"robots\" content=\"noindex,nofollow\" />\n");
    }
    html = html.replace("</head>", &format!("{metadata}</head>"));

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"))
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        )
        .body(Body::from(html))
        .expect("index response should be valid")
}

fn is_private_page(path: &str) -> bool {
    path == "/admin"
        || path.starts_with("/admin/")
        || path == "/login"
        || path.starts_with("/login/")
        || path == "/account"
        || path.starts_with("/account/")
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn normalize_path(path: &str) -> String {
    path.trim_start_matches('/')
        .split('/')
        .filter(|part| !part.is_empty() && *part != "." && *part != "..")
        .collect::<Vec<_>>()
        .join("/")
}

fn asset_response(path: &str, bytes: &'static [u8]) -> Response {
    let content_type = mime_guess::from_path(path).first_or_octet_stream();
    let cache_control = if matches!(
        path,
        "index.html"
            | "favicon.svg"
            | "favicon.ico"
            | "favicon-32x32.png"
            | "apple-touch-icon.png"
            | "icon-192.png"
            | "icon-512.png"
            | "icon-512-maskable.png"
            | "site.webmanifest"
    ) {
        HeaderValue::from_static("no-cache")
    } else {
        HeaderValue::from_static("public, max-age=31536000, immutable")
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CACHE_CONTROL, cache_control)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_str(content_type.as_ref())
                .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
        )
        .body(Body::from(bytes))
        .expect("static asset response should be valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn branded_icons_manifest_and_touch_metadata_are_embedded_and_revalidated() {
        let site = SiteConfig::default();
        for path in [
            "favicon.svg",
            "favicon.ico",
            "favicon-32x32.png",
            "apple-touch-icon.png",
            "icon-192.png",
            "icon-512.png",
            "icon-512-maskable.png",
            "site.webmanifest",
        ] {
            let file = WEB_DIST
                .get_file(path)
                .expect("brand asset must be embedded");
            let response = serve(&format!("/{path}"), &site, "https://mirror.example");
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-cache");
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            assert_eq!(body.as_ref(), file.contents());
        }
        let html = b"<html><head><title>MirrorProxy</title><link rel=\"icon\" href=\"/favicon.svg\"><link rel=\"apple-touch-icon\" href=\"/apple-touch-icon.png\"></head></html>";
        for (site, expected) in [
            (site, "/apple-touch-icon.png"),
            (
                SiteConfig {
                    icon_url: "/custom.svg".into(),
                    ..SiteConfig::default()
                },
                "/custom.svg",
            ),
        ] {
            let response = index_response(html, "/", &site, "https://mirror.example");
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let body = std::str::from_utf8(&body).unwrap();
            assert_eq!(body.matches("rel=\"apple-touch-icon\"").count(), 1);
            assert!(body.contains(&format!("rel=\"apple-touch-icon\" href=\"{expected}\"")));
        }
    }

    #[tokio::test]
    async fn renders_metadata_in_minified_umi_html_and_marks_nested_pages_private() {
        let site = SiteConfig {
            title: "Mirror <One>".to_string(),
            icon_url: "/brand.svg".to_string(),
            ..SiteConfig::default()
        };
        let response = index_response(b"<!doctype html><html><head><title>MirrorProxy</title><link rel=\"icon\" href=\"/favicon.svg\"></head><body></body></html>", "/admin/downloads", &site, "https://mirror.example");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-cache");
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = std::str::from_utf8(&body).unwrap();
        assert!(body.contains("<title>Mirror &lt;One&gt;</title>"));
        assert!(body.contains("href=\"/brand.svg\""));
        assert!(body.contains("noindex,nofollow"));
        assert!(body.contains("https://mirror.example"));
    }

    #[tokio::test]
    async fn same_origin_installers_are_plain_text_current_scripts_without_cache() {
        for (path, expected) in [
            (
                "/install.sh",
                include_bytes!("../../../scripts/install.sh").as_slice(),
            ),
            (
                "/install.ps1",
                include_bytes!("../../../scripts/install.ps1").as_slice(),
            ),
        ] {
            let response = serve(path, &SiteConfig::default(), "https://mirror.example");
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(
                response.headers()[header::CONTENT_TYPE],
                "text/plain; charset=utf-8"
            );
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-cache");
            assert_eq!(response.headers()["x-content-type-options"], "nosniff");
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            assert_eq!(body.as_ref(), expected);
        }
    }

    #[test]
    fn missing_umi_chunks_are_not_served_as_html() {
        let response = serve(
            "/missing-chunk.123456.js",
            &SiteConfig::default(),
            "https://mirror.example",
        );
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
