use tracing::{error, info};

use crate::logging;
use crate::settings_window::bridge::lib::{
  BridgeHttpResponse, BridgeRequest, ResolvedRoute, make_error,
  success_response,
};

#[derive(Debug, serde::Serialize)]
struct SettingsOpenAboutLogsDirResponse {
  logs_dir: String,
}

pub fn handle(
  req: &BridgeRequest,
  route: &ResolvedRoute,
) -> BridgeHttpResponse {
  info!(
    request_id = ?req.request_id,
    route = %route.route_kind,
    "settings open about logs dir request"
  );

  match logging::open_logs_dir_in_file_manager() {
    Ok(logs_dir) => success_response(
      req.request_id.clone(),
      route,
      SettingsOpenAboutLogsDirResponse {
        logs_dir: logs_dir.display().to_string(),
      },
    ),
    Err(err) => {
      error!(
        request_id = ?req.request_id,
        error = %err,
        "failed to open logs directory"
      );

      let error = make_error(
        "OPEN_LOGS_DIR_FAILED",
        format!("Failed to open logs directory: {err}"),
        None,
        None,
        Some(err),
      );

      let body = serde_json::json!({
        "request_id": req.request_id.clone(),
        "ok": false,
        "kind": "error.open_logs_dir_failed",
        "payload": {},
        "error": error,
      })
      .to_string();

      BridgeHttpResponse { status: 500, body }
    }
  }
}
