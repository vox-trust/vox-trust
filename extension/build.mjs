// Builds the extension for each browser from src/: dist/chrome (Chrome, Edge, Opera, Brave),
// dist/firefox and dist/safari (input for Apple's converter), plus a zip of each.
// The WebAssembly core and its wrapper come from web/ (run scripts/build-web.sh first).
// Usage: node extension/build.mjs
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");
const wasm = join(root, "web/vox_trust.wasm");
if (!existsSync(wasm)) throw new Error("web/vox_trust.wasm missing: run scripts/build-web.sh");
const version = JSON.parse(readFileSync(join(root, "npm/package.json"), "utf8")).version;

const base = {
  manifest_version: 3,
  name: "__MSG_extName__",
  description: "__MSG_extDescription__",
  version,
  default_locale: "en",
  icons: { 16: "icons/icon-16.png", 32: "icons/icon-32.png", 48: "icons/icon-48.png", 128: "icons/icon-128.png" },
  action: { default_popup: "popup.html", default_icon: { 16: "icons/icon-16.png", 32: "icons/icon-32.png" } },
  permissions: ["contextMenus", "storage"],
  // Asked one site at a time, only when you verify a link from that site.
  optional_host_permissions: ["https://*/*", "http://*/*"],
  content_security_policy: { extension_pages: "script-src 'self' 'wasm-unsafe-eval'; object-src 'self'" },
  homepage_url: "https://github.com/vox-trust/vox-trust",
};

const targets = {
  chrome: { ...base, background: { service_worker: "background.js" }, minimum_chrome_version: "116" },
  firefox: {
    ...base,
    background: { scripts: ["background.js"] },
    browser_specific_settings: { gecko: {
        id: "vox-trust@vox-trust.github.io",
        strict_min_version: "128.0",
        // Nothing leaves the browser.
        data_collection_permissions: { required: ["none"] },
      },
    },
  },
  safari: { ...base, background: { service_worker: "background.js" } },
};

mkdirSync(join(here, "dist"), { recursive: true });
for (const [name, manifest] of Object.entries(targets)) {
  const out = join(here, "dist", name);
  rmSync(out, { recursive: true, force: true });
  cpSync(join(here, "src"), out, { recursive: true });
  cpSync(join(root, "web/vox-trust.js"), join(out, "vox-trust.js"));
  cpSync(wasm, join(out, "vox_trust.wasm"));
  cpSync(join(root, "LICENSE"), join(out, "LICENSE"));
  writeFileSync(join(out, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
  const zip = join(here, "dist", `vox-trust-${name}-${version}.zip`);
  rmSync(zip, { force: true });
  execFileSync("zip", ["-qrX", zip, "."], { cwd: out });
  console.log(`${name}: dist/${name} and ${zip.slice(here.length + 1)}`);
}
