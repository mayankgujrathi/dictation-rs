use serde::Deserialize;

use crate::decode_payload;

use crate::settings_window::bridge::lib::{
  BridgeHttpResponse, BridgeRequest, ResolvedRoute, success_response,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsPingRequest {
  source: String,
}

#[derive(Debug, serde::Serialize)]
struct SettingsPingResponse {
  pong: bool,
  source: String,
}

pub fn handle(
  req: &BridgeRequest,
  route: &ResolvedRoute,
) -> BridgeHttpResponse {
  let payload: SettingsPingRequest =
    match decode_payload!(req, SettingsPingRequest, &route.route_kind) {
      Ok(payload) => payload,
      Err(response) => return response,
    };

  success_response(
    req.request_id.clone(),
    route,
    SettingsPingResponse {
      pong: true,
      source: payload.source,
    },
  )
}
