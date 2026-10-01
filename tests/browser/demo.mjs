// Drives the real demo page in headless Chromium: seal, attack, verify, pin keys.
// Run: scripts/build-web.sh && node tests/browser/demo.mjs
// Needs Playwright with Chromium installed. Optional: SHOTS=/some/dir to save screenshots.
import assert from "node:assert/strict";
import { createReadStream, existsSync, mkdirSync } from "node:fs";
import { createServer } from "node:http";
import { dirname, extname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

const here = dirname(fileURLToPath(import.meta.url));
const webDir = resolve(here, "../../web");
const shots = process.env.SHOTS;
if (shots) mkdirSync(shots, { recursive: true });

const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".wasm": "application/wasm", ".svg": "image/svg+xml" };
const server = createServer((req, res) => {
  const path = join(webDir, req.url === "/" ? "index.html" : decodeURIComponent(req.url.split("?")[0]));
  if (!path.startsWith(webDir) || !existsSync(path)) {
    res.writeHead(404).end("not found");
    return;
  }
  res.writeHead(200, { "content-type": types[extname(path)] || "application/octet-stream" });
  createReadStream(path).pipe(res);
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const base = `http://127.0.0.1:${server.address().port}/`;

const browser = await chromium.launch();
const problems = [];
let passed = 0;
const step = async (name, fn) => {
  try {
    await fn();
    passed++;
    console.log(`ok   ${name}`);
  } catch (error) {
    problems.push(`${name}: ${error.message.split("\n")[0]}`);
    console.log(`FAIL ${name}\n     ${error.message.split("\n")[0]}`);
  }
};

const context = await browser.newContext({ viewport: { width: 1280, height: 1000 } });
const page = await context.newPage();
const pageErrors = [];
page.on("pageerror", (e) => pageErrors.push(e.message));
page.on("console", (m) => m.type() === "error" && pageErrors.push(m.text()));
await page.goto(base);

const title = () => page.locator("#vTitle").innerText();
const verdictClass = () => page.locator("#verdict").getAttribute("class");
const chunkClasses = () => page.locator("#chunks li").evaluateAll((els) => els.map((e) => e.className));
const shot = async (name) => shots && (await page.screenshot({ path: join(shots, `${name}.png`), fullPage: true }));

await step("the WebAssembly core loads and a sample clip is ready", async () => {
  await page.waitForFunction(() => document.getElementById("coreStatus").textContent.startsWith("Core ready"));
  assert.match(await page.locator("#audioInfo").innerText(), /Sample clip: 6\.0 s/);
  const inked = await page.locator("#wave").evaluate((c) => {
    const d = c.getContext("2d").getImageData(0, 0, c.width, c.height).data;
    let n = 0;
    for (let i = 3; i < d.length; i += 4) if (d[i] > 0) n++;
    return n;
  });
  assert.ok(inked > 500, `waveform should be drawn (${inked} pixels)`);
});

await step("circle mode: sealing verifies, and all 6 chunks are intact", async () => {
  await page.click("#sealBtn");
  await page.waitForFunction(() => document.getElementById("vTitle").textContent === "Verified");
  assert.match(await verdictClass(), /verified/);
  assert.deepEqual(await chunkClasses(), Array(6).fill("ok"));
  assert.match(await page.locator("#sealInfo").innerText(), /6 chunks/);
  await shot("1-verified");
});

await step("replacing a chunk with silence is caught and localised to that chunk", async () => {
  await page.selectOption("#atkChunk", "2");
  await page.click('[data-attack="silence"]');
  await page.waitForFunction(() => document.getElementById("vTitle").textContent.includes("changed"));
  assert.match(await verdictClass(), /alert/);
  assert.deepEqual(await chunkClasses(), ["ok", "ok", "bad", "ok", "ok", "ok"]);
  assert.match(await page.locator("#vText").innerText(), /chunk: 2\./);
  await shot("2-tampered");
});

await step("flipping a single bit is caught", async () => {
  await page.click('[data-attack="reset"]');
  await page.waitForFunction(() => document.getElementById("vTitle").textContent === "Verified");
  await page.selectOption("#atkChunk", "4");
  await page.click('[data-attack="flip"]');
  await page.waitForFunction(() => document.getElementById("vTitle").textContent.includes("changed"));
  assert.deepEqual(await chunkClasses(), ["ok", "ok", "ok", "ok", "bad", "ok"]);
});

await step("swapping two chunks is caught (position is bound)", async () => {
  await page.click('[data-attack="reset"]');
  await page.selectOption("#atkChunk", "0");
  await page.click('[data-attack="swap"]');
  await page.waitForFunction(() => document.getElementById("vTitle").textContent.includes("changed"));
  assert.deepEqual(await chunkClasses(), ["bad", "bad", "ok", "ok", "ok", "ok"]);
});

await step("a lossy conversion breaks file mode on every chunk (the honest limitation)", async () => {
  await page.click('[data-attack="reset"]');
  await page.click('[data-attack="lossy"]');
  await page.waitForFunction(() => document.getElementById("vTitle").textContent.includes("changed"));
  assert.deepEqual(await chunkClasses(), Array(6).fill("bad"));
});

await step("stripping the seal: the verdict depends on who you think it is", async () => {
  await page.click('[data-attack="reset"]');
  await page.click('[data-attack="strip"]');
  await page.waitForFunction(() => document.getElementById("vTitle").textContent === "Unsealed");
  assert.match(await verdictClass(), /unsealed/);
  await page.selectOption("#contact", "always");
  await page.waitForFunction(() => document.getElementById("vTitle").textContent.startsWith("No seal, from someone"));
  assert.match(await verdictClass(), /warning/);
  await page.selectOption("#contact", "strict");
  await page.waitForFunction(() => document.getElementById("vTitle").textContent.includes("strict mode"));
  assert.match(await verdictClass(), /alert/);
  await shot("3-stripped-strict");
  await page.selectOption("#contact", "stranger");
});

await step("a wrong passphrase on the verifier does not verify", async () => {
  await page.click('[data-attack="reset"]');
  await page.fill("#verifyPass", "a different passphrase");
  await page.press("#verifyPass", "Tab");
  await page.waitForFunction(() => document.getElementById("vTitle").textContent === "Unsealed");
  assert.match(await page.locator("#vText").innerText(), /key you don't trust/);
  await page.fill("#verifyPass", "demo passphrase");
  await page.press("#verifyPass", "Tab");
  await page.waitForFunction(() => document.getElementById("vTitle").textContent === "Verified");
});

await step("circle mode: an attacker re-sealing with their own key is rejected", async () => {
  await page.click('[data-attack="reseal"]');
  await page.waitForFunction(() => document.getElementById("vTitle").textContent.includes("does not match your key"));
  assert.match(await verdictClass(), /alert/);
});

await step("public mode: key pair, seal, verify with the pinned key", async () => {
  await page.check('input[name="mode"][value="public"]');
  await page.click("#genKeys");
  assert.match(await page.locator("#pinned").inputValue(), /^[0-9a-f]{64}$/);
  await page.click("#sealBtn");
  await page.waitForFunction(() => document.getElementById("vTitle").textContent === "Verified");
  assert.deepEqual(await chunkClasses(), Array(6).fill("ok"));
  await shot("4-public-verified");
});

await step("public mode: an attacker's own valid signature is not trusted by a pinned contact", async () => {
  await page.click('[data-attack="reseal"]');
  await page.waitForFunction(() => document.getElementById("vTitle").textContent === "Unsealed");
  assert.match(await page.locator("#vText").innerText(), /key you don't trust/);
  await page.selectOption("#contact", "always");
  await page.waitForFunction(() => document.getElementById("vTitle").textContent.includes("different key"));
  assert.match(await verdictClass(), /alert/);
  await page.selectOption("#contact", "stranger");
});

await step("public mode: trust on first use, pinning the embedded key", async () => {
  await page.click('[data-attack="reset"]');
  await page.click("#forgetPin");
  await page.waitForFunction(() => document.getElementById("vTitle").textContent === "Unsealed");
  assert.deepEqual(await chunkClasses(), Array(6).fill("unknown"));
  await page.locator("#detailsBox summary").click();
  assert.equal(await page.locator("#pinEmbedded").isVisible(), true);
  await page.click("#pinEmbedded");
  await page.waitForFunction(() => document.getElementById("vTitle").textContent === "Verified");
});

await step("the download link serves a WAV file named for the verdict", async () => {
  const href = await page.locator("#download").getAttribute("href");
  assert.match(href, /^blob:/);
  assert.equal(await page.locator("#download").getAttribute("download"), "sealed.wav");
  // The page CSP (connect-src 'self') blocks fetch(blob:), so use a real download instead.
  const [download] = await Promise.all([page.waitForEvent("download"), page.click("#download")]);
  assert.equal(download.suggestedFilename(), "sealed.wav");
  const chunks = [];
  for await (const c of await download.createReadStream()) chunks.push(c);
  assert.deepEqual([...Buffer.concat(chunks).subarray(0, 4)], [0x52, 0x49, 0x46, 0x46]); // RIFF
});

await step("a tampered file is never offered as sealed.wav", async () => {
  await page.selectOption("#atkChunk", "1");
  await page.click('[data-attack="silence"]');
  await page.waitForFunction(() => document.getElementById("vTitle").textContent.includes("changed"));
  assert.equal(await page.locator("#download").getAttribute("download"), "unverified.wav");
  await page.click('[data-attack="reset"]');
  await page.waitForFunction(() => document.getElementById("vTitle").textContent === "Verified");
});

await step("a malformed pasted public key gets feedback and does not crash", async () => {
  await page.fill("#pinned", "zz-not-a-key");
  await page.press("#pinned", "Tab");
  await page.waitForFunction(() => document.getElementById("pinInfo").textContent.includes("Not a valid public key"));
  assert.equal(await page.locator("#pinned").getAttribute("aria-invalid"), "true");
  assert.notEqual(await title(), "Verified"); // the pinned key is ignored, so a public seal is untrusted
});

await step("the file pickers are keyboard reachable with a visible focus ring", async () => {
  await page.focus("#verifyFile");
  const outline = await page.locator("label.btn:has(#verifyFile)").evaluate((el) => getComputedStyle(el).outlineStyle);
  assert.notEqual(outline, "none");
  assert.equal(await page.locator("#verifyFile").evaluate((el) => el.hidden), false);
});

await step("Forget keys clears the keys and passphrases, and verification stops trusting", async () => {
  await page.click("#forgetKeys");
  await page.waitForFunction(() => document.getElementById("vTitle").textContent === "Unsealed");
  assert.equal(await page.locator("#pinned").inputValue(), "");
  assert.equal(await page.locator("#verifyPass").inputValue(), "");
});

await step("every button, input and select has an accessible name", async () => {
  const unnamed = await page.evaluate(() =>
    [...document.querySelectorAll("button, input:not([hidden]), select")]
      .filter((el) => el.type !== "hidden")
      .filter((el) => {
        const label = el.labels && el.labels.length ? el.labels[0].textContent.trim() : "";
        return !(el.textContent.trim() || label || el.getAttribute("aria-label") || el.getAttribute("title"));
      })
      .map((el) => el.id || el.outerHTML.slice(0, 60)),
  );
  assert.deepEqual(unnamed, []);
});

await step("no horizontal scrolling at phone width, and the page still works", async () => {
  const phone = await browser.newContext({ viewport: { width: 375, height: 800 }, deviceScaleFactor: 2 });
  const p = await phone.newPage();
  await p.goto(base);
  await p.waitForFunction(() => document.getElementById("coreStatus").textContent.startsWith("Core ready"));
  await p.click("#sealBtn");
  await p.waitForFunction(() => document.getElementById("vTitle").textContent === "Verified");
  const overflow = await p.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  assert.ok(overflow <= 0, `horizontal overflow of ${overflow}px`);
  if (shots) await p.screenshot({ path: join(shots, "5-phone.png"), fullPage: true });
  await phone.close();
});

await step("dark mode renders without errors", async () => {
  const dark = await browser.newContext({ viewport: { width: 1000, height: 900 }, colorScheme: "dark" });
  const p = await dark.newPage();
  await p.goto(base);
  await p.waitForFunction(() => document.getElementById("coreStatus").textContent.startsWith("Core ready"));
  await p.click("#sealBtn");
  await p.waitForFunction(() => document.getElementById("vTitle").textContent === "Verified");
  if (shots) await p.screenshot({ path: join(shots, "6-dark.png"), fullPage: true });
  await dark.close();
});

await step("no console errors or uncaught exceptions during the whole session", async () => {
  assert.deepEqual(pageErrors, []);
});

await browser.close();
server.close();
console.log(`\n${passed} passed, ${problems.length} failed`);
if (problems.length) {
  console.log(problems.map((p) => ` - ${p}`).join("\n"));
  process.exit(1);
}
