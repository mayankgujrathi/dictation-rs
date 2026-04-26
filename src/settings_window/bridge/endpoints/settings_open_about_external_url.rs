use serde::Deserialize;
use tracing::{error, info};

use crate::decode_payload;
use crate::logging;
use crate::settings_window::bridge::lib::{
  BridgeHttpResponse, BridgeRequest, ResolvedRoute, make_error,
  success_response,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsOpenAboutExternalUrlRequest {
  url: String,
}

#[derive(Debug, serde::Serialize)]
struct SettingsOpenAboutExternalUrlResponse {
  url: String,
}

fn is_allowed_url(url: &str) -> bool {
  url.starts_with("https://") || url.starts_with("http://")
}

pub fn handle(
  req: &BridgeRequest,
  route: &ResolvedRoute,
) -> BridgeHttpResponse {
  info!(
    request_id = ?req.request_id,
    route = %route.route_kind,
    "settings open about external url request"
  );

  let payload: SettingsOpenAboutExternalUrlRequest = match decode_payload!(
    req,
    SettingsOpenAboutExternalUrlRequest,
    &route.route_kind
  ) {
    Ok(payload) => payload,
    Err(response) => return response,
  };

  if !is_allowed_url(&payload.url) {
    let error = make_error(
      "INVALID_URL",
      "Only http:// and https:// URLs are allowed",
      Some("url"),
      Some("http://... or https://..."),
      Some(payload.url),
    );
    let body = serde_json::json!({
      "request_id": req.request_id.clone(),
      "ok": false,
      "kind": "error.invalid_url",
      "payload": {},
      "error": error,
    })
    .to_string();

    return BridgeHttpResponse { status: 400, body };
  }

  match logging::open_url_in_default_browser(&payload.url) {
    Ok(()) => success_response(
      req.request_id.clone(),
      route,
      SettingsOpenAboutExternalUrlResponse { url: payload.url },
    ),
    Err(err) => {
      error!(
        request_id = ?req.request_id,
        error = %err,
        "failed to open about external url"
      );
      let error = make_error(
        "OPEN_EXTERNAL_URL_FAILED",
        format!("Failed to open URL in browser: {err}"),
        Some("url"),
        Some("http://... or https://..."),
        Some(payload.url),
      );
      let body = serde_json::json!({
        "request_id": req.request_id.clone(),
        "ok": false,
        "kind": "error.open_external_url_failed",
        "payload": {},
        "error": error,
      })
      .to_string();

      BridgeHttpResponse { status: 500, body }
    }
  }
}
