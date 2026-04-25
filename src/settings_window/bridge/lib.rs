use serde::Deserialize;

pub type IpcRequest = wry::http::Request<String>;

#[derive(Debug, Clone, serde::Serialize)]
pub struct BridgeHttpResponse {
  pub status: u16,
  pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct BridgeRequest {
  #[serde(default)]
  pub request_id: Option<String>,
  #[serde(default)]
  pub method: Option<String>,
  #[serde(default)]
  pub endpoint: Option<String>,
  #[serde(default)]
  pub payload: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct ResolvedRoute {
  pub endpoint: String,
  pub route_kind: String,
}

pub type RouteHandler =
  fn(&BridgeRequest, &ResolvedRoute) -> BridgeHttpResponse;

#[derive(Clone, Copy)]
pub struct RouteDef {
  pub method: &'static str,
  pub endpoint: &'static str,
  pub handler: RouteHandler,
}

#[derive(Debug, serde::Serialize)]
pub struct BridgeError {
  code: String,
  message: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  field: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  expected: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  received: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct BridgeResponse {
  request_id: Option<String>,
  ok: bool,
  kind: String,
  payload: serde_json::Value,
  #[serde(skip_serializing_if = "Option::is_none")]
  error: Option<BridgeError>,
}

pub fn make_error(
  code: &str,
  message: impl Into<String>,
  field: Option<&str>,
  expected: Option<&str>,
  received: Option<String>,
) -> BridgeError {
  BridgeError {
    code: code.to_string(),
    message: message.into(),
    field: field.map(str::to_string),
    expected: expected.map(str::to_string),
    received,
  }
}

pub fn extract_missing_field(err: &str) -> Option<String> {
  let marker = "missing field `";
  let start = err.find(marker)? + marker.len();
  let end = err[start..].find('`')? + start;
  Some(err[start..end].to_string())
}

pub fn invalid_payload_error(
  route_kind: &str,
  request_id: Option<String>,
  err: &str,
) -> BridgeHttpResponse {
  let (code, field, expected) = if err.contains("missing field") {
    (
      "MISSING_FIELD",
      extract_missing_field(err),
      Some("required field".to_string()),
    )
  } else if err.contains("unknown field") {
    (
      "UNKNOWN_FIELD",
      None,
      Some("only documented payload fields".to_string()),
    )
  } else if err.contains("invalid type") {
    (
      "INVALID_FIELD_TYPE",
      None,
      Some("correct field type".to_string()),
    )
  } else {
    ("INVALID_PAYLOAD", None, None)
  };

  let message = format!("Invalid payload for route '{route_kind}': {err}");
  let response = BridgeResponse {
    request_id,
    ok: false,
    kind: "error.invalid_payload".to_string(),
    payload: serde_json::json!({}),
    error: Some(make_error(
      code,
      message,
      field.as_deref(),
      expected.as_deref(),
      Some(err.to_string()),
    )),
  };

  let body = serde_json::to_string(&response).unwrap_or_else(|_| {
    "{\"ok\":false,\"kind\":\"error.serialize\",\"error\":{\"code\":\"SERIALIZE_ERROR\",\"message\":\"failed to serialize error response\"}}".to_string()
  });
  BridgeHttpResponse { status: 400, body }
}

pub fn json_decode_error(raw_body: &str, err: &str) -> BridgeHttpResponse {
  let response = BridgeResponse {
    request_id: None,
    ok: false,
    kind: "error.invalid_json".to_string(),
    payload: serde_json::json!({}),
    error: Some(make_error(
      "INVALID_JSON",
      format!("Request body is not valid JSON: {err}"),
      None,
      Some("JSON object with request_id, method, endpoint, payload"),
      Some(raw_body.to_string()),
    )),
  };
  let body = serde_json::to_string(&response).unwrap_or_else(|_| {
    "{\"ok\":false,\"kind\":\"error.serialize\",\"error\":{\"code\":\"SERIALIZE_ERROR\",\"message\":\"failed to serialize error response\"}}".to_string()
  });
  BridgeHttpResponse { status: 400, body }
}

pub fn unknown_route_error(
  method: &str,
  endpoint: &str,
  request_id: Option<String>,
  supported_routes: &str,
) -> BridgeHttpResponse {
  let response = BridgeResponse {
    request_id,
    ok: false,
    kind: "error.unknown_route".to_string(),
    payload: serde_json::json!({}),
    error: Some(make_error(
      "UNKNOWN_ROUTE",
      format!("Unsupported IPC route '{method} {endpoint}'"),
      Some("endpoint"),
      Some(supported_routes),
      Some(format!("{method} {endpoint}")),
    )),
  };
  let body = serde_json::to_string(&response).unwrap_or_else(|_| {
    "{\"ok\":false,\"kind\":\"error.serialize\",\"error\":{\"code\":\"SERIALIZE_ERROR\",\"message\":\"failed to serialize error response\"}}".to_string()
  });
  BridgeHttpResponse { status: 400, body }
}

pub fn missing_route_error(request_id: Option<String>) -> BridgeHttpResponse {
  let response = BridgeResponse {
    request_id,
    ok: false,
    kind: "error.missing_route".to_string(),
    payload: serde_json::json!({}),
    error: Some(make_error(
      "MISSING_ROUTE",
      "Request must provide method + endpoint",
      Some("method/endpoint"),
      Some("method='POST', endpoint='/settings/ping'"),
      None,
    )),
  };
  let body = serde_json::to_string(&response).unwrap_or_else(|_| {
    "{\"ok\":false,\"kind\":\"error.serialize\",\"error\":{\"code\":\"SERIALIZE_ERROR\",\"message\":\"failed to serialize error response\"}}".to_string()
  });
  BridgeHttpResponse { status: 400, body }
}

pub fn success_response(
  request_id: Option<String>,
  route: &ResolvedRoute,
  payload: impl serde::Serialize,
) -> BridgeHttpResponse {
  let payload =
    serde_json::to_value(payload).unwrap_or_else(|_| serde_json::json!({}));
  let response = BridgeResponse {
    request_id,
    ok: true,
    kind: success_kind_from_route(route),
    payload,
    error: None,
  };
  let body = serde_json::to_string(&response).unwrap_or_else(|_| {
    "{\"ok\":false,\"kind\":\"error.serialize\",\"error\":{\"code\":\"SERIALIZE_ERROR\",\"message\":\"failed to serialize success response\"}}".to_string()
  });
  BridgeHttpResponse { status: 200, body }
}

pub fn success_kind_from_route(route: &ResolvedRoute) -> String {
  let kind = route.endpoint.trim_start_matches('/').replace('/', ".");
  format!("{kind}.reply")
}

pub fn normalize_endpoint(endpoint: &str) -> String {
  if endpoint.starts_with('/') {
    endpoint.to_string()
  } else {
    format!("/{endpoint}")
  }
}

pub fn find_route<'a>(
  routes: &'a [RouteDef],
  method: &str,
  endpoint: &str,
) -> Option<&'a RouteDef> {
  routes
    .iter()
    .find(|route| route.method == method && route.endpoint == endpoint)
}

pub fn supported_routes_text(routes: &[RouteDef]) -> String {
  routes
    .iter()
    .map(|route| format!("{} {}", route.method, route.endpoint))
    .collect::<Vec<_>>()
    .join(" or ")
}

#[macro_export]
macro_rules! decode_payload {
  ($req:expr, $ty:ty, $route_kind:expr) => {
    serde_json::from_value::<$ty>($req.payload.clone()).map_err(|e| {
      let err_string = e.to_string();
      $crate::settings_window::bridge::lib::invalid_payload_error(
        $route_kind,
        $req.request_id.clone(),
        &err_string,
      )
    })
  };
}

#[macro_export]
macro_rules! route_table {
  ($(($method:expr, $endpoint:expr, $handler:path)),+ $(,)?) => {
    &[
      $(
        $crate::settings_window::bridge::lib::RouteDef {
          method: $method,
          endpoint: $endpoint,
          handler: $handler,
        }
      ),+
    ]
  };
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(serde::Serialize)]
  struct DummyPayload {
    ok: bool,
  }

  fn dummy_handler(
    _req: &BridgeRequest,
    _route: &ResolvedRoute,
  ) -> BridgeHttpResponse {
    BridgeHttpResponse {
      status: 200,
      body: "{}".to_string(),
    }
  }

  #[test]
  fn success_kind_derived_from_endpoint() {
    let route = ResolvedRoute {
      endpoint: "/settings/ping".to_string(),
      route_kind: "POST /settings/ping".to_string(),
    };

    assert_eq!(success_kind_from_route(&route), "settings.ping.reply");
  }

  #[test]
  fn success_response_uses_route_derived_kind() {
    let route = ResolvedRoute {
      endpoint: "/settings/concat".to_string(),
      route_kind: "POST /settings/concat".to_string(),
    };

    let response = success_response(
      Some("rid-1".to_string()),
      &route,
      DummyPayload { ok: true },
    );

    assert_eq!(response.status, 200);
    assert!(response.body.contains("settings.concat.reply"));
    assert!(response.body.contains("\"ok\":true"));
  }

  #[test]
  fn supported_routes_text_lists_all_routes() {
    let routes = [
      RouteDef {
        method: "POST",
        endpoint: "/settings/ping",
        handler: dummy_handler,
      },
      RouteDef {
        method: "POST",
        endpoint: "/settings/concat",
        handler: dummy_handler,
      },
    ];

    assert_eq!(
      supported_routes_text(&routes),
      "POST /settings/ping or POST /settings/concat"
    );
  }
}
