//! The dashboard, served by the master itself so a browser pointed at
//! `http://<master>:7878` is all a user needs.
//!
//! The files are the Svelte build (`apps/desktop/dist`), compiled into the
//! binary with the `embed-ui` feature. Without it (plain `cargo build`, the
//! tests) `/` explains how to get a build that has it.

use axum::{
    http::{
        HeaderValue, StatusCode, Uri,
        header::{CACHE_CONTROL, CONTENT_SECURITY_POLICY, CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS},
    },
    response::{IntoResponse, Response},
};

/// No other site may frame the dashboard (its Approve button must not be
/// clickjacked). Scripts and styles come only from the master. `connect-src`
/// stays open because the same page can connect to another master. WebKit
/// (Safari, and the desktop app on macOS and Linux) does not count `ws:` as
/// part of `*`, so the live updates socket needs naming.
const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; \
                   img-src 'self' data:; connect-src * ws: wss:; frame-ancestors 'none'; \
                   base-uri 'none'; form-action 'none'";

fn with_headers(mut response: Response, content_type: &str, immutable: bool) -> Response {
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(content_type) {
        headers.insert(CONTENT_TYPE, value);
    }
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(CONTENT_SECURITY_POLICY, HeaderValue::from_static(CSP));
    // Vite puts a content hash in asset names, so those never change; the
    // page itself must be fetched fresh to pick up a new build.
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static(if immutable {
            "public, max-age=31536000, immutable"
        } else {
            "no-cache"
        }),
    );
    response
}

/// Serves a dashboard file for any path the API does not claim.
pub async fn serve(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    // Unknown API or socket paths are errors, not the page.
    if path.starts_with("api/") || path.starts_with("ws/") {
        return StatusCode::NOT_FOUND.into_response();
    }
    asset(if path.is_empty() { "index.html" } else { path })
}

#[cfg(feature = "embed-ui")]
#[derive(rust_embed::Embed)]
#[folder = "../../apps/desktop/dist"]
struct Dashboard;

#[cfg(feature = "embed-ui")]
fn asset(path: &str) -> Response {
    match Dashboard::get(path) {
        Some(file) => with_headers(
            file.data.into_owned().into_response(),
            file.metadata.mimetype(),
            path.starts_with("assets/"),
        ),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[cfg(not(feature = "embed-ui"))]
fn asset(path: &str) -> Response {
    if path != "index.html" {
        return StatusCode::NOT_FOUND.into_response();
    }
    let page = "<!doctype html><meta charset=utf-8><title>Nodeloom</title>\
        <p>This master was built without the dashboard. Use a release build \
        (it serves the dashboard here), or run the desktop app from \
        <code>apps/desktop</code> and connect it to this address.</p>";
    with_headers(page.into_response(), "text/html; charset=utf-8", false)
}

/// Whether this binary carries the dashboard.
pub const EMBEDDED: bool = cfg!(feature = "embed-ui");
