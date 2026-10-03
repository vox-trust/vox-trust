// Loads the built Chrome extension in Chromium and drives the verifier page: adding
// contacts, the four verdicts, tamper localization and the link flow.
// Run: scripts/build-web.sh && node extension/build.mjs && node tests/extension/extension.mjs
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { encodeWav, hex, loadVoxTrust } from "../../web/vox-trust.js";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const ext = join(root, "extension/dist/chrome");
const vt = await loadVoxTrust(readFileSync(join(ext, "vox_trust.wasm")));
const work = mkdtempSync(join(tmpdir(), "vt-ext-"));

const samples = Int16Array.from({ length: 3 * 16000 }, (_, i) => Math.round(8000 * Math.sin(i / 7)));
const wav = encodeWav(samples, 16000);
const seed = new Uint8Array(32).fill(5);
const { publicKey } = vt.publicKey(seed);
const sealed = vt.seal(wav, { mode: "public", key: seed, createdUnix: 1790000000, chunkFrames: 16000 });
const tampered = sealed.slice();
tampered[44 + 2 * 20000] ^= 0x40; // second 1
const stranger = vt.seal(wav, { mode: "public", key: new Uint8Array(32).fill(9), createdUnix: 1790000000, chunkFrames: 16000 });
const files = { sealed, tampered, stranger, plain: wav };
for (const [name, bytes] of Object.entries(files)) writeFileSync(join(work, `${name}.wav`), bytes);

let failures = 0;
const step = async (name, fn) => {
  try {
    await fn();
    console.log(`ok   ${name}`);
  } catch (e) {
    failures++;
    console.log(`FAIL ${name}\n     ${e.message.split("\n")[0]}`);
  }
};

const context = await chromium.launchPersistentContext(join(work, "profile"), {
  channel: "chromium",
  headless: true,
  args: [`--disable-extensions-except=${ext}`, `--load-extension=${ext}`],
});
let [worker] = context.serviceWorkers();
worker ??= await context.waitForEvent("serviceworker");
const id = new URL(worker.url()).host;
const page = await context.newPage();
const errors = [];
page.on("pageerror", (e) => errors.push(e.message));

const verdict = async (file, sender = "") => {
  await page.selectOption("#sender", sender);
  await page.setInputFiles("#file", join(work, `${file}.wav`));
  await page.waitForFunction(() => !document.getElementById("verdict").hidden);
  await page.waitForTimeout(100);
  return page.evaluate(() => ({
    kind: document.getElementById("verdict").className.replace("verdict ", ""),
    title: document.getElementById("vTitle").textContent,
    text: document.getElementById("vText").textContent,
    chunks: document.getElementById("vChunks").textContent,
  }));
};

await step("the popup opens and is translated", async () => {
  const popup = await context.newPage();
  await popup.goto(`chrome-extension://${id}/popup.html`);
  assert.equal(await popup.textContent("#open"), "Verify a file");
  await popup.close();
});

await page.goto(`chrome-extension://${id}/verify.html`);

await step("an invalid pairing text is refused", async () => {
  await page.fill("#pairing", `voxtrust:0:public:${hex(publicKey).toUpperCase()}`);
  await page.click("#addForm button[type=submit]");
  assert.equal(await page.textContent("#formMsg"), "That is not a valid pairing text.");
});

await step("a public pairing text adds a contact with its label", async () => {
  await page.fill("#pairing", `voxtrust:0:public:${hex(publicKey)}:Ana%20Souza`);
  await page.check("#always");
  await page.click("#addForm button[type=submit]");
  await page.waitForSelector("#contactList li");
  assert.match(await page.textContent("#contactList li"), /Ana Souza/);
});

await step("a file sealed by the contact is verified", async () => {
  const v = await verdict("sealed");
  assert.equal(v.kind, "verified");
  assert.match(v.text, /Ana Souza/);
});

await step("an edited file is an alert that names the changed second", async () => {
  const v = await verdict("tampered");
  assert.equal(v.kind, "alert");
  assert.match(v.chunks, /1\.0–2\.0 s/);
  if (process.env.SCREENSHOT) await page.screenshot({ path: process.env.SCREENSHOT, fullPage: true });
});

await step("no seal from a contact who always seals is a warning", async () => {
  const v = await verdict("plain", "0");
  assert.equal(v.kind, "warning");
});

await step("no seal and no claimed sender is neutral", async () => {
  const v = await verdict("plain", "");
  assert.equal(v.kind, "neutral");
  assert.equal(v.title, "No seal");
});

await step("a stranger's seal is neutral and says the audio is unchanged", async () => {
  const v = await verdict("stranger", "");
  assert.equal(v.kind, "neutral");
  assert.match(v.text, /not pinned, and unchanged/);
});

await step("a stranger's seal claimed to come from the contact is an alert", async () => {
  const v = await verdict("stranger", "0");
  assert.equal(v.kind, "alert");
});

await step("contacts persist across reloads", async () => {
  await page.reload();
  await page.waitForSelector("#contactList li");
  assert.match(await page.textContent("#contactList li"), /Ana Souza/);
});

await step("a link asks for access to that one site before downloading", async () => {
  const server = createServer((_, res) => res.end(Buffer.from(sealed))).listen(0);
  const port = server.address().port;
  await page.goto(`chrome-extension://${id}/verify.html?src=${encodeURIComponent(`http://127.0.0.1:${port}/a.wav`)}`);
  await page.waitForSelector("#allow:not([hidden])");
  assert.match(await page.textContent("#fetchText"), new RegExp(`http://127\\.0\\.0\\.1:${port}`));
  server.close();
});

await step("no uncaught errors", async () => assert.deepEqual(errors, []));

await context.close();
rmSync(work, { recursive: true, force: true });
console.log(failures ? `\n${failures} failed` : "\nall passed");
process.exit(failures ? 1 : 0);
