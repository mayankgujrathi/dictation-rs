const responseEl = document.getElementById("response");

const sendToRust = async (message) => {
  if (!window.ipc?.postMessage) {
    throw new Error("window.ipc.postMessage is not available");
  }

  const requestId =
    message?.request_id ||
    (crypto?.randomUUID ? crypto.randomUUID() : String(Date.now()));
  const payload = { ...message, request_id: requestId };

  window.ipc.postMessage(JSON.stringify(payload));
  return {
    request_id: requestId,
    ok: true,
    kind: `${payload.kind}.queued`,
    payload: {
      sent_to_rust: true,
      via: "window.ipc.postMessage",
      echo: payload.payload
    }
  };
};

document.getElementById("ping")?.addEventListener("click", async () => {
  try {
    const reply = await sendToRust({
      request_id: crypto?.randomUUID ? crypto.randomUUID() : String(Date.now()),
      kind: "settings.ping",
      payload: { source: "settings-ui" }
    });
    responseEl.textContent = JSON.stringify(reply, null, 2);
  } catch (error) {
    responseEl.textContent = `Request failed: ${String(error)}`;
  }
});