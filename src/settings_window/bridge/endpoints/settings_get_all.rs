use tracing::{debug, info};

use crate::settings;
use crate::settings_window::bridge::lib::{
  BridgeHttpResponse, BridgeRequest, ResolvedRoute, success_response,
};

#[derive(Debug, serde::Serialize)]
struct SettingsGetAllResponse {
  settings: settings::AppSettings,
}

pub fn handle(
  req: &BridgeRequest,
  route: &ResolvedRoute,
) -> BridgeHttpResponse {
  info!(
    request_id = ?req.request_id,
    route = %route.route_kind,
    "settings get-all request"
  );
  let current = settings::current();
  debug!(
    request_id = ?req.request_id,
    start_on_login = current.start_on_login,
    "settings get-all response payload prepared"
  );
  success_response(
    req.request_id.clone(),
    route,
    SettingsGetAllResponse { settings: current },
  )
}
