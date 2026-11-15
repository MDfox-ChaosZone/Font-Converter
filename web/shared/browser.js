const input = document.createElement("input");
input.type = "file";
input.accept = ".ttf,.otf,.woff2";
input.hidden = true;
document.body.append(input);

let result;
let pending = false;
let message = "Select a font to convert locally.";

function discard() {
  if (result) URL.revokeObjectURL(result.url);
  result = undefined;
}
window.addEventListener("pagehide", event => { if (!event.persisted) discard(); });
input.addEventListener("cancel", () => { pending = false; });
input.addEventListener("change", () => {
  const file = input.files[0];
  if (!file) { pending = false; return; }
  discard();
  const worker = new Worker(new URL("./worker.js", import.meta.url));
  function finish(error) {
    if (error) message = String(error);
    pending = false;
    worker.terminate();
  }
  worker.onerror = event => finish(event.message || "Conversion worker failed");
  worker.onmessage = ({data}) => {
    if (data.error) { finish(data.error); return; }
    const name = file.name.replace(/\.[^.]+$/, "") + "." + data.extension;
    result = {name, url: URL.createObjectURL(new Blob([data.bytes], {type:"font/" + data.extension}))};
    message = name + "  ·  " + data.bytes.length.toLocaleString() + " bytes";
    finish();
  };
  message = "Converting " + file.name + "…";
  worker.postMessage(file);
});

export const fontConverter = {
  choose() {
    if (pending) return;
    pending = true;
    input.value = "";
    try { input.click(); } catch (error) { pending = false; message = error.message; }
  },
  pending: () => pending,
  ready: () => Boolean(result),
  status: () => message,
  download() {
    if (!result || pending) return;
    const link = document.createElement("a");
    link.href = result.url;
    link.download = result.name;
    link.click();
  },
};
globalThis.fontConverter = fontConverter;
