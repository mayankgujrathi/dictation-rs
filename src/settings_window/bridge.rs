use serde::Deserialize;
use tracing::{debug, warn};
use wry::WebView;

type IpcRequest = wry::http::Request<String>;

#[derive(Debug, Deserialize)]
pub struct BridgeRequest {
  #[serde(default)]
  pub request_id: Option<String>,
  pub kind: String,
  #[serde(default)]
  pub payload: serde_json::Value,
}

#[derive(Debug, serde::Serialize)]
struct BridgeResponse {
  request_id: Option<String>,
  ok: bool,
  kind: String,
  payload: serde_json::Value,
}

pub fn handle_ipc(request: IpcRequest) {
  let payload = request.body();
  match serde_json::from_str::<BridgeRequest>(payload) {
    Ok(message) => {
      debug!(
        kind = %message.kind,
        payload = %message.payload,
        request_id = ?message.request_id,
        "settings window IPC bridge request received"
      );
    }
    Err(e) => {
      warn!(error = %e, raw = %payload, "failed to parse settings IPC message");
    }
  }
}

pub fn handle_bridge_request(raw_body: &str) -> String {
  match serde_json::from_str::<BridgeRequest>(raw_body) {
    Ok(req) => {
      debug!(kind = %req.kind, payload = %req.payload, "settings bridge request received");

      let response_kind = format!("{}.reply", req.kind);
      let response = BridgeResponse {
        request_id: req.request_id,
        ok: true,
        kind: response_kind,
        payload: serde_json::json!({
          "echo": req.payload,
          "handled_by": "settings_window.bridge",
          "timestamp_ms": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
        }),
      };

      serde_json::to_string(&response).unwrap_or_else(|_| {
        "{\"ok\":false,\"kind\":\"error.serialize\"}".to_string()
      })
    }
    Err(e) => {
      warn!(error = %e, raw = %raw_body, "failed to parse settings bridge request");
      let response = BridgeResponse {
        request_id: None,
        ok: false,
        kind: "error.invalid_json".to_string(),
        payload: serde_json::json!({ "message": e.to_string() }),
      };
      serde_json::to_string(&response).unwrap_or_else(|_| {
        "{\"ok\":false,\"kind\":\"error.serialize\"}".to_string()
      })
    }
  }
}

/// Future-facing helper for Rust -> JS communication.
///
/// You can call this with a snippet such as:
/// `window.dispatchEvent(new CustomEvent('settings:update', { detail: ... }))`
/// once the UI starts listening for these events.
pub fn eval_js(webview: &WebView, script: &str) {
  if let Err(e) = webview.evaluate_script(script) {
    warn!(error = %e, "failed to evaluate script in settings webview");
  }
}
