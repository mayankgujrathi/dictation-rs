use tracing::info;

use crate::settings;
use crate::settings_window::bridge::lib::{
  BridgeHttpResponse, BridgeRequest, ResolvedRoute, success_response,
};

#[derive(Debug, serde::Serialize)]
struct SettingsGetLoggingResponse {
  logging: settings::LoggingSettings,
}

pub fn handle(
  req: &BridgeRequest,
  route: &ResolvedRoute,
) -> BridgeHttpResponse {
  info!(
    request_id = ?req.request_id,
    route = %route.route_kind,
    "settings get logging request"
  );
  success_response(
    req.request_id.clone(),
    route,
    SettingsGetLoggingResponse {
      logging: settings::current().logging,
    },
  )
}
