use tracing::info;

use crate::logging;
use crate::settings_window::bridge::lib::{
  BridgeHttpResponse, BridgeRequest, ResolvedRoute, success_response,
};

#[derive(Debug, serde::Serialize)]
struct SettingsGetAboutLogsDirResponse {
  logs_dir: String,
}

pub fn handle(
  req: &BridgeRequest,
  route: &ResolvedRoute,
) -> BridgeHttpResponse {
  info!(
    request_id = ?req.request_id,
    route = %route.route_kind,
    "settings get about logs dir request"
  );

  let logs_dir = logging::logs_dir_path();
  success_response(
    req.request_id.clone(),
    route,
    SettingsGetAboutLogsDirResponse {
      logs_dir: logs_dir.display().to_string(),
    },
  )
}
