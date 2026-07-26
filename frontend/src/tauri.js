export function invokeCommand(command, args) {
  return window.__TAURI__.core.invoke(command, args);
}
export function listenProgress(callback) {
  return window.__TAURI__.event.listen("conversion-progress", (event) => {
    callback(event.payload);
  });
}

export function listenDragDrop(callback) {
  return window.__TAURI__.webview
    .getCurrentWebview()
    .onDragDropEvent((event) => {
      if (event.payload.type === "drop") {
        callback(event.payload.paths);
      }
    });
}
