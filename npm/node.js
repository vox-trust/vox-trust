// Node: fetch() cannot read file: URLs, so the bundled WebAssembly file is read from disk.
import { readFile } from "node:fs/promises";
import { loadVoxTrust } from "./vox-trust.js";

export * from "./vox-trust.js";

/** Loads the bundled WebAssembly core, or the one at `source` (bytes or URL) if given. */
export async function load(source) {
  return loadVoxTrust(source ?? (await readFile(new URL("./vox_trust.wasm", import.meta.url))));
}
