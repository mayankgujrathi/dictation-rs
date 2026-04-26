use tracing::info;

use crate::settings;
use crate::settings_window::bridge::lib::{
  BridgeHttpResponse, BridgeRequest, ResolvedRoute, success_response,
};

#[derive(Debug, serde::Serialize)]
struct SettingsGetStartOnLoginResponse {
  start_on_login: bool,
}

pub fn handle(
  req: &BridgeRequest,
  route: &ResolvedRoute,
) -> BridgeHttpResponse {
  info!(
    request_id = ?req.request_id,
    route = %route.route_kind,
    "settings get start_on_login request"
  );
  success_response(
    req.request_id.clone(),
    route,
    SettingsGetStartOnLoginResponse {
      start_on_login: settings::current().start_on_login,
    },
  )
}
