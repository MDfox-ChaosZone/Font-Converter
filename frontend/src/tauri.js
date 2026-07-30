export function invokeCommand(command, args) {
  if (!window.__TAURI__?.core) {
    return Promise.reject(new Error("Tauri is unavailable in this browser preview"));
  }
  return window.__TAURI__.core.invoke(command, args);
}
export function listenProgress(callback) {
  if (!window.__TAURI__?.event) {
    return Promise.resolve(() => {});
  }
  return window.__TAURI__.event.listen("conversion-progress", (event) => {
    callback(event.payload);
  });
}

export function listenDragDrop(callback) {
  if (!window.__TAURI__?.webview) {
    return Promise.resolve(() => {});
  }
  return window.__TAURI__.webview
    .getCurrentWebview()
    .onDragDropEvent((event) => {
      callback({
        kind: event.payload.type,
        paths: event.payload.paths ?? [],
      });
    });
}
