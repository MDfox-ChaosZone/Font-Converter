// All font bytes stay in this worker. Each job owns a fresh codec instance.
self.onmessage = async ({ data: file }) => {
  try {
    const maxInput = 256 * 1024 * 1024;
    if (file.size > maxInput) throw new Error("Input exceeds 256 MiB");
    const input = new Uint8Array(await file.arrayBuffer());
    const tag = offset => String.fromCharCode(...input.subarray(offset, offset + 4));
    const sfnt = value => ["OTTO", "\0\x01\0\0", "true"].includes(value);
    const decode = tag(0) === "wOF2";
    if (input.length < (decode ? 48 : 12) || !sfnt(tag(decode ? 4 : 0))) {
      throw new Error("Choose a TTF, OTF or WOFF2 font (collections are unsupported)");
    }
    const extension = decode ? (tag(4) === "OTTO" ? "otf" : "ttf") : "woff2";
    const wasi = {
      fd_write: () => 8, fd_close: () => 8, fd_seek: () => 8, // WASI EBADF: no open descriptors
      proc_exit: () => { throw new Error("Codec stopped while reading this font"); },
    };
    const { instance } = await WebAssembly.instantiateStreaming(
      fetch(new URL("./codec.wasm", self.location.href)), { wasi_snapshot_preview1: wasi });
    const codec = instance.exports;
    codec._initialize();
    const allocate = size => {
      const address = codec.malloc(size);
      if (!address) throw new Error("Not enough browser memory for this font");
      return address;
    };
    const source = allocate(input.length);
    new Uint8Array(codec.memory.buffer, source, input.length).set(input);
    const capacity = codec.fc_capacity(source, input.length, +!decode) >>> 0;
    if (!capacity || capacity > (decode ? 128 * 1024 * 1024 : 2 * maxInput)) {
      throw new Error("Invalid or excessive output size");
    }
    const output = allocate(capacity);
    const written = allocate(4);
    if (!codec.fc_convert(source, input.length, output, capacity, written, +!decode)) {
      throw new Error("Google WOFF2 rejected the font");
    }
    const length = new DataView(codec.memory.buffer).getUint32(written, true);
    if (!length || length > capacity) throw new Error("Invalid codec result");
    const bytes = new Uint8Array(codec.memory.buffer, output, length).slice();
    self.postMessage({ bytes, extension }, [bytes.buffer]);
  } catch (error) {
    self.postMessage({ error: String(error.message || error) });
  }
};
