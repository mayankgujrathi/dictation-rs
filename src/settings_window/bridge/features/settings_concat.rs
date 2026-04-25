use serde::Deserialize;

use crate::decode_payload;

use crate::settings_window::bridge::lib::{
  BridgeHttpResponse, BridgeRequest, ResolvedRoute, success_response,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsConcatRequest {
  x: i64,
  y: String,
}

#[derive(Debug, serde::Serialize)]
struct SettingsConcatResponse {
  result: String,
}

pub fn handle(
  req: &BridgeRequest,
  route: &ResolvedRoute,
) -> BridgeHttpResponse {
  let payload: SettingsConcatRequest =
    match decode_payload!(req, SettingsConcatRequest, &route.route_kind) {
      Ok(payload) => payload,
      Err(response) => return response,
    };

  success_response(
    req.request_id.clone(),
    route,
    SettingsConcatResponse {
      result: format!("{}{}", payload.x, payload.y),
    },
  )
}
