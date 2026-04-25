const responseEl = document.getElementById("response");
const concatXEl = document.getElementById("concat-x");
const concatYEl = document.getElementById("concat-y");

/**
 * @typedef {{ request_id?: string, method: "GET"|"POST"|"PUT"|"DELETE"|"PATCH", endpoint: string, payload: unknown }} IpcRequest
 */

/**
 * @typedef {{ code?: string, message?: string, field?: string, expected?: string, received?: string }} IpcError
 */

/**
 * @typedef {{ request_id?: string, ok: boolean, kind: string, payload: unknown, error?: IpcError }} IpcResponse
 */

const makeRequestId = () =>
  crypto?.randomUUID ? crypto.randomUUID() : String(Date.now());

const normalizeError = (errorLike) => {
  if (typeof errorLike === "string") {
    return { code: "UNKNOWN", message: errorLike };
  }
  if (!errorLike || typeof errorLike !== "object") {
    return { code: "UNKNOWN", message: "Unknown IPC error" };
  }

  return {
    code: errorLike.code || "UNKNOWN",
    message: errorLike.message || "IPC request failed",
    field: errorLike.field,
    expected: errorLike.expected,
    received:
      typeof errorLike.received === "string"
        ? errorLike.received
        : errorLike.received != null
          ? JSON.stringify(errorLike.received)
          : undefined
  };
};

const formatErrorForUi = (error) => {
  const lines = [
    `code: ${error.code || "UNKNOWN"}`,
    `message: ${error.message || "IPC request failed"}`
  ];
  if (error.field) lines.push(`field: ${error.field}`);
  if (error.expected) lines.push(`expected: ${error.expected}`);
  if (error.received) lines.push(`received: ${error.received}`);
  return lines.join("\n");
};

/**
 * Sends a typed IPC request to Rust using WebView IPC transport.
 * Note: Fetch with custom dictation:// scheme is not supported in this runtime.
 * @param {IpcRequest} request
 * @returns {Promise<IpcResponse>}
 */
const sendIpc = async (request) => {
  const request_id = request.request_id || makeRequestId();
  const body = {
    request_id,
    method: request.method,
    endpoint: request.endpoint,
    payload: request.payload
  };

  return await new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open("POST", "/ipc", true);
    xhr.setRequestHeader("content-type", "application/json");
    xhr.timeout = 10000;

    xhr.onload = () => {
      let parsed = null;
      try {
        parsed = JSON.parse(xhr.responseText || "{}");
      } catch {
        reject(
          new Error(
            `IPC response was not valid JSON (status=${xhr.status}, request_id=${request_id})`
          )
        );
        return;
      }

      if (xhr.status >= 400 || !parsed?.ok) {
        const normalized = normalizeError(parsed?.error || parsed);
        const error = new Error(formatErrorForUi(normalized));
        error.ipc = { status: xhr.status, response: parsed, normalized };
        reject(error);
        return;
      }

      resolve(parsed);
    };

    xhr.onerror = () => {
      reject(new Error(`IPC request failed (network error, request_id=${request_id})`));
    };

    xhr.ontimeout = () => {
      reject(new Error(`IPC request timed out for request_id=${request_id}`));
    };

    xhr.send(JSON.stringify(body));
  });
};

const setResponse = (value) => {
  responseEl.textContent =
    typeof value === "string" ? value : JSON.stringify(value, null, 2);
};

document.getElementById("ping")?.addEventListener("click", async () => {
  try {
    const reply = await sendIpc({
      method: "POST",
      endpoint: "/settings/ping",
      payload: { source: "settings-ui" }
    });
    setResponse(reply);
  } catch (error) {
    setResponse(`Request failed:\n${String(error)}`);
  }
});

document.getElementById("concat")?.addEventListener("click", async () => {
  try {
    const x = Number(concatXEl?.value);
    const y = String(concatYEl?.value || "");
    const reply = await sendIpc({
      method: "POST",
      endpoint: "/settings/concat",
      payload: { x, y }
    });
    setResponse(reply);
  } catch (error) {
    setResponse(`Request failed:\n${String(error)}`);
  }
});