use tracing::info;

use crate::settings_window::bridge::lib::{
  BridgeHttpResponse, BridgeRequest, ResolvedRoute, success_response,
};

#[derive(Debug, serde::Serialize)]
struct SettingsWindowReadyResponse {
  visible: bool,
}

pub fn handle(
  req: &BridgeRequest,
  route: &ResolvedRoute,
) -> BridgeHttpResponse {
  info!(
    request_id = ?req.request_id,
    route = %route.route_kind,
    "settings window ready signal received"
  );

  crate::settings_window::mark_settings_window_ui_ready();

  success_response(
    req.request_id.clone(),
    route,
    SettingsWindowReadyResponse { visible: true },
  )
}
