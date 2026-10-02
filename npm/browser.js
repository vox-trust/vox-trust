// Browsers and bundlers: the WebAssembly file ships next to this module, and bundlers that
// understand `new URL(..., import.meta.url)` (Vite, webpack 5, esbuild, Parcel) copy it.
import { loadVoxTrust } from "./vox-trust.js";

export * from "./vox-trust.js";

/** Loads the bundled WebAssembly core, or the one at `source` (bytes or URL) if given. */
export function load(source) {
  return loadVoxTrust(source ?? new URL("./vox_trust.wasm", import.meta.url));
}
