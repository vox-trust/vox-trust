// Drives the real demo page in headless Chromium: seal, attack, verify, pin keys.
// Run: scripts/build-web.sh && node tests/browser/demo.mjs
// Needs Playwright with Chromium installed. Optional: SHOTS=/some/dir to save screenshots.
import assert from "node:assert/strict";
import { createReadStream, existsSync, mkdirSync } from "node:fs";
import { createServer } from "node:http";
import { dirname, extname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

import { encodeWav } from "../../web/vox-trust.js";

const here = dirname(fileURLToPath(import.meta.url));
const webDir = resolve(here, "../../web");
const shots = process.env.SHOTS;
if (shots) mkdirSync(shots, { recursive: true });

const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".wasm": "application/wasm", ".svg": "image/svg+xml" };
const server = createServer((req, res) => {
  const pathname = decodeURIComponent(req.url.split("?")[0]);
  const path = join(webDir, pathname === "/" ? "index.html" : pathname);
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
  assert.equal(await page.locator("canvas, #vIcon, .step, img, svg").count(), 0, "no decorative icons, badges or images");
});

await step("headings carry their step number as plain text; the verdict is title text plus colour", async () => {
  assert.deepEqual(await page.locator("h2").allInnerTexts(), ["1 · Pick some audio", "2 · Seal it", "3 · Try to cheat", "4 · Verify"]);
  assert.match(await page.locator("#audioInfo").innerText(), /6\.0 s, 16 kHz mono/);
  assert.equal(await page.locator("#verdict > *").count(), 2, "verdict is only a title and a text");
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

// ---------------------------------------------------------------- languages

const LANGS = {
  en: { dir: "ltr", h1: "Seal it. Try to cheat. Watch it catch you.", ok: "Verified", alert: "Alert: the audio was changed", label: "Language", title: "Vox Trust demo: seal a voice, try to cheat it, verify in your browser", chunk: "altered" },
  "pt-BR": { dir: "ltr", h1: "Sele. Tente fraudar. Veja o selo te pegar.", ok: "Verificado", alert: "Alerta: o áudio foi alterado", label: "Idioma", title: "Demo do Vox Trust: sele uma voz, tente fraudá-la, verifique no navegador", chunk: "alterado" },
  es: { dir: "ltr", h1: "Séllalo. Intenta engañar. Mira cómo te descubre.", ok: "Verificado", alert: "Alerta: el audio fue modificado", label: "Idioma", title: "Demo de Vox Trust: sella una voz, intenta engañarla, verifica en tu navegador", chunk: "alterado" },
  "zh-Hans": { dir: "ltr", h1: "签章。试着作弊。看它如何识破你。", ok: "已验证", alert: "警报：音频已被改动", label: "语言", title: "Vox Trust 演示：为语音签章，试着作弊，在浏览器中验证", chunk: "已改动" },
  ar: { dir: "rtl", h1: "اختمه. حاول التلاعب. وشاهد الختم يكشفك.", ok: "تم التحقق", alert: "تنبيه: تم تغيير الصوت", label: "اللغة", title: "عرض Vox Trust التجريبي: اختم صوتًا، وحاول التلاعب به، وتحقق في متصفحك", chunk: "معدَّل" },
};
const htmlAttrs = (p) => p.evaluate(() => ({ lang: document.documentElement.lang, dir: document.documentElement.dir }));
const pickLang = async (p, code) => {
  await p.selectOption("#lang", code);
  await p.waitForFunction((c) => document.documentElement.lang === c, code);
};
const ready = (p) => p.waitForFunction(() => !document.getElementById("sealBtn").disabled);

const lctx = await browser.newContext({ viewport: { width: 1280, height: 1000 } });
const lp = await lctx.newPage();
const lTitle = () => lp.locator("#vTitle").innerText();
const lClass = () => lp.locator("#verdict").getAttribute("class");
const lChunks = () => lp.locator("#chunks li").evaluateAll((els) => els.map((e) => e.className));
lp.on("pageerror", (e) => pageErrors.push(e.message));
lp.on("console", (m) => m.type() === "error" && pageErrors.push(m.text()));
await lp.goto(base);
await ready(lp);
await lp.click("#sealBtn");
await lp.waitForFunction(() => document.getElementById("vTitle").textContent === "Verified");

await step("the language selector lists the five native names and is labelled", async () => {
  const names = await lp.locator("#lang option").allInnerTexts();
  assert.deepEqual(names, ["English", "Português (Brasil)", "Español", "中文(简体)", "العربية"]);
  assert.equal(await lp.locator("#lang").getAttribute("aria-label"), "Language");
  assert.deepEqual(await htmlAttrs(lp), { lang: "en", dir: "ltr" });
});

await step("each language sets <html lang> and dir, translates the page, and re-renders the verdict", async () => {
  for (const [code, want] of Object.entries(LANGS)) {
    await pickLang(lp, code);
    assert.deepEqual(await htmlAttrs(lp), { lang: code, dir: want.dir }, code);
    assert.equal(await lp.locator("h1").innerText(), want.h1, code);
    assert.equal(await lTitle(), want.ok, `${code} verdict title`);
    assert.equal(await lp.locator("#lang").getAttribute("aria-label"), want.label, code);
    assert.equal(await lp.title(), want.title, code);
    assert.match(await lClass(), /verified/);
  }
  await pickLang(lp, "en");
});

await step("title and meta description follow the language, and the URL gets ?lang=", async () => {
  await pickLang(lp, "es");
  assert.match(await lp.locator('meta[name="description"]').getAttribute("content"), /^Sella un fragmento de voz/);
  assert.equal(new URL(lp.url()).searchParams.get("lang"), "es");
  await pickLang(lp, "en");
  assert.equal(new URL(lp.url()).searchParams.get("lang"), "en");
  assert.match(await lp.locator('meta[name="description"]').getAttribute("content"), /^Seal a voice clip/);
  assert.equal(await lp.locator('link[rel="canonical"]').getAttribute("href"), "https://vox-trust.github.io/demo/", "canonical stays the bare URL");
});

await step("switching language re-renders a tampered verdict from the stored report, without re-verifying", async () => {
  await lp.selectOption("#atkChunk", "2");
  await lp.click('[data-attack="silence"]');
  await lp.waitForFunction(() => document.getElementById("vTitle").textContent.includes("changed"));
  // setVerdictBox rewrites the verdict's class, so any (re-)verification would show up here.
  await lp.evaluate(() => {
    window.__verdicts = [];
    new MutationObserver(() => window.__verdicts.push(document.getElementById("verdict").className)).observe(document.getElementById("verdict"), { attributes: true });
  });
  for (const [code, want] of Object.entries(LANGS)) {
    await pickLang(lp, code);
    assert.equal(await lTitle(), want.alert, code);
    assert.match(await lClass(), /alert/);
    assert.deepEqual(await lChunks(), ["ok", "ok", "bad", "ok", "ok", "ok"], code);
    assert.match(await lp.locator("#vText").innerText(), /2/);
    assert.match(await lp.locator("#chunks li").nth(2).innerText(), new RegExp(want.chunk), code);
    assert.match(await lp.locator("#chunks li").nth(2).getAttribute("aria-label"), /2/);
  }
  assert.deepEqual(await lp.evaluate(() => window.__verdicts), [], "no verification ran");
  await pickLang(lp, "en");
});

await step("RTL: Arabic isolates numbers, hex and the report so they stay left-to-right", async () => {
  await pickLang(lp, "ar");
  assert.equal(await lp.locator("#report").evaluate((e) => getComputedStyle(e).direction), "ltr");
  assert.equal(await lp.locator("#pinned").evaluate((e) => getComputedStyle(e).direction), "ltr");
  assert.equal(await lp.locator("body").evaluate((e) => getComputedStyle(e).direction), "rtl");
  assert.match(await lp.locator("#coreStatus").innerText(), /\u2068/);
  await pickLang(lp, "en");
});

await step("the sealed-and-verified flow still works end to end in Chinese and Arabic", async () => {
  await pickLang(lp, "zh-Hans");
  await lp.click("#sealBtn");
  await lp.waitForFunction(() => document.getElementById("vTitle").textContent === "已验证");
  assert.match(await lp.locator("#sealInfo").innerText(), /已用圈子模式签章：6 个片段/);
  await pickLang(lp, "ar");
  await lp.click('[data-attack="strip"]');
  await lp.waitForFunction(() => document.getElementById("vTitle").textContent === "غير مختوم");
  assert.match(await lClass(), /unsealed/);
  await pickLang(lp, "en");
  assert.equal(await lTitle(), "Unsealed");
});

await step("a malformed key message and the copy button follow the language", async () => {
  await lp.fill("#pinned", "zz");
  await lp.press("#pinned", "Tab");
  await lp.waitForFunction(() => document.getElementById("pinInfo").textContent.includes("Not a valid public key"));
  await pickLang(lp, "pt-BR");
  assert.match(await lp.locator("#pinInfo").innerText(), /^Chave pública inválida/);
  await pickLang(lp, "en");
  assert.match(await lp.locator("#pinInfo").innerText(), /^Not a valid public key/);
});

await step("?lang=es on load selects Spanish; ?lang beats storage; a bad value is ignored", async () => {
  const p = await lctx.newPage();
  await p.goto(`${base}?lang=es`);
  await ready(p);
  assert.deepEqual(await htmlAttrs(p), { lang: "es", dir: "ltr" });
  assert.equal(await p.locator("#lang").inputValue(), "es");
  assert.equal(await p.locator("h1").innerText(), LANGS.es.h1);
  await p.waitForFunction(() => document.getElementById("coreStatus").textContent.startsWith("Núcleo listo"));
  assert.match(await p.locator("#audioInfo").innerText(), /^Fragmento de muestra: 6,0 s/);
  // ?lang= is a one-off: it must not overwrite the saved preference.
  await p.goto(base);
  assert.equal((await htmlAttrs(p)).lang, "en");
  await p.goto(`${base}?lang=xx`);
  assert.equal((await htmlAttrs(p)).lang, "en");
  await p.close();
});

await step("the chosen language persists across reloads (localStorage) and ?lang= wins over it", async () => {
  await pickLang(lp, "pt-BR");
  const p = await lctx.newPage();
  await p.goto(base);
  await ready(p);
  assert.equal((await htmlAttrs(p)).lang, "pt-BR");
  assert.equal(await p.locator("h1").innerText(), LANGS["pt-BR"].h1);
  await p.goto(`${base}?lang=ar`);
  assert.deepEqual(await htmlAttrs(p), { lang: "ar", dir: "rtl" });
  await p.close();
  await pickLang(lp, "en");
});

await step("with nothing stored, navigator.languages picks the language (en otherwise)", async () => {
  for (const [locale, want] of [["zh-CN", "zh-Hans"], ["ar-EG", "ar"], ["pt-PT", "pt-BR"], ["es-MX", "es"], ["fr-FR", "en"], ["zh-TW", "en"]]) {
    const c = await browser.newContext({ locale });
    const p = await c.newPage();
    await p.goto(base);
    assert.equal((await htmlAttrs(p)).lang, want, locale);
    await c.close();
  }
});

await step("a blocked localStorage does not break language selection", async () => {
  const c = await browser.newContext();
  await c.addInitScript(() => {
    Object.defineProperty(window, "localStorage", { get() { throw new Error("blocked"); } });
  });
  const p = await c.newPage();
  const errs = [];
  p.on("pageerror", (e) => errs.push(e.message));
  await p.goto(base);
  await ready(p);
  await pickLang(p, "es");
  assert.equal(await p.locator("h1").innerText(), LANGS.es.h1);
  assert.deepEqual(errs, []);
  await c.close();
});

await step("Arabic and Chinese layouts have no horizontal overflow at 375px and 1280px", async () => {
  for (const code of ["ar", "zh-Hans", "es"]) {
    for (const width of [375, 1280]) {
      const c = await browser.newContext({ viewport: { width, height: 900 }, deviceScaleFactor: 1 });
      const p = await c.newPage();
      await p.goto(`${base}?lang=${code}`);
      await ready(p);
      await p.click("#sealBtn");
      await p.waitForFunction(() => document.querySelectorAll("#chunks li.ok").length === 6);
      await p.selectOption("#atkChunk", "1");
      await p.click('[data-attack="silence"]');
      await p.waitForFunction(() => document.querySelectorAll("#chunks li.bad").length === 1);
      const overflow = await p.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
      assert.ok(overflow <= 0, `${code}@${width}: horizontal overflow of ${overflow}px`);
      if (shots) await p.screenshot({ path: join(shots, `lang-${code}-${width}.png`), fullPage: true });
      await c.close();
    }
  }
});

// ---------------------------------------------------------------- head, hostile input, failures

await step("the head is complete, JSON-LD parses, and the page raised no CSP error for it", async () => {
  const head = await page.evaluate(() => ({
    canonical: document.querySelector('link[rel="canonical"]').href,
    robots: document.querySelector('meta[name="robots"]').content,
    ogImage: document.querySelector('meta[property="og:image"]').content,
    card: document.querySelector('meta[name="twitter:card"]').content,
    themes: [...document.querySelectorAll('meta[name="theme-color"]')].map((m) => m.media),
    icons: [...document.querySelectorAll('link[rel~="icon"], link[rel="apple-touch-icon"]')].map((l) => l.getAttribute("href")),
    ld: JSON.parse(document.querySelector('script[type="application/ld+json"]').textContent),
    scheme: getComputedStyle(document.documentElement).colorScheme,
  }));
  assert.equal(head.canonical, "https://vox-trust.github.io/demo/");
  assert.equal(head.robots, "index,follow");
  assert.equal(head.ogImage, "https://vox-trust.github.io/og.png");
  assert.equal(head.card, "summary_large_image");
  assert.equal(head.themes.length, 2);
  assert.deepEqual(head.icons, ["/favicon.svg", "/favicon.ico", "/apple-touch-icon.png"]);
  assert.equal(head.ld["@type"], "WebApplication");
  assert.equal(head.scheme, "light dark");
});

await step("English loads eagerly and other languages load only when chosen", async () => {
  const c = await browser.newContext();
  const p = await c.newPage();
  const loaded = [];
  p.on("request", (r) => /\/i18n\/[\w-]+\.js$/.test(r.url()) && loaded.push(r.url().split("/").pop()));
  await p.goto(base);
  await ready(p);
  assert.deepEqual(loaded.sort(), ["en.js", "index.js"]);
  await pickLang(p, "ar");
  assert.ok(loaded.includes("ar.js"));
  assert.ok(!loaded.includes("zh-Hans.js") && !loaded.includes("es.js"));
  assert.equal(await p.locator("h1").innerText(), LANGS.ar.h1);
  await c.close();
});

await step("a hostile file name renders inertly: no bidi override, no placeholder or markdown expansion, layout intact", async () => {
  const name = "evil\u202E{abi}**<b>x</b>.wav";
  const wav = Buffer.from(encodeWav(new Int16Array(16000).fill(1000), 16000, 1));
  for (const code of ["en", "ar"]) {
    const c = await browser.newContext({ viewport: { width: 375, height: 900 } });
    const p = await c.newPage();
    await p.goto(`${base}?lang=${code}`);
    await ready(p);
    await p.setInputFiles("#pickFile", { name, mimeType: "audio/wav", buffer: wav });
    await p.waitForFunction(() => document.getElementById("audioInfo").textContent.includes("evil"));
    const info = await p.locator("#audioInfo").evaluate((e) => ({ text: e.textContent, kids: e.children.length, w: e.getBoundingClientRect().width }));
    assert.ok(info.text.startsWith("\u2068evil{abi}**<b>x</b>.wav\u2069"), `${code}: ${JSON.stringify(info.text)}`);
    assert.ok(!info.text.includes("\u202E"), "override character stripped");
    assert.equal(info.kids, 0, "no elements created from the name");
    assert.ok(info.w <= 375);
    await p.click("#sealBtn");
    await p.waitForFunction(() => document.querySelectorAll("#chunks li.ok").length === 1);
    assert.ok((await p.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)) <= 0, `${code}: overflow`);
    if (shots) await p.screenshot({ path: join(shots, `hostile-name-${code}.png`), fullPage: true });
    await c.close();
  }
});

await step("markup inside a dictionary value stays inert text, rich strings keep only bold and known links", async () => {
  const c = await browser.newContext();
  const p = await c.newPage();
  await p.goto(`${base}?lang=es`);
  await ready(p);
  await p.evaluate(async () => {
    const dict = (await import("/i18n/es.js")).default;
    dict["hero.title"] = '<img src=x onerror="window.__pwn=1"><script>window.__pwn=2<\/script>';
    dict["hero.lead"] = 'a <img src=y> **bold** [kept words](nope) [home](src) <b>tag</b>';
  });
  await pickLang(p, "en");
  await pickLang(p, "es");
  assert.equal(await p.locator("h1").textContent(), '<img src=x onerror="window.__pwn=1"><script>window.__pwn=2</script>');
  assert.equal(await p.locator("h1 > *").count(), 0);
  const lead = p.locator(".lead");
  assert.equal(await lead.locator("img, b, script").count(), 0);
  assert.equal(await lead.locator("strong").innerText(), "bold");
  assert.equal(await lead.locator("a").count(), 1, "only the known link id becomes a link");
  assert.match(await lead.textContent(), /kept words/);
  assert.match(await lead.textContent(), /<b>tag<\/b>/);
  assert.equal(await p.evaluate(() => window.__pwn), undefined);
  await c.close();
});

await step("attacks use the demo's own chunk size, even after verifying a file sealed with another one", async () => {
  const c = await browser.newContext({ viewport: { width: 1280, height: 1000 } });
  const p = await c.newPage();
  await p.goto(base);
  await ready(p);
  await p.selectOption("#chunkSize", "32000");
  await p.click("#sealBtn");
  await p.waitForFunction(() => document.querySelectorAll("#chunks li.ok").length === 3);
  const [download] = await Promise.all([p.waitForEvent("download"), p.click("#download")]);
  const parts = [];
  for await (const part of await download.createReadStream()) parts.push(part);
  const foreign = Buffer.concat(parts);
  await p.selectOption("#chunkSize", "16000");
  await p.click("#sealBtn");
  await p.waitForFunction(() => document.querySelectorAll("#chunks li.ok").length === 6);
  await p.setInputFiles("#verifyFile", { name: "foreign.wav", mimeType: "audio/wav", buffer: foreign });
  await p.waitForFunction(() => document.querySelectorAll("#chunks li").length === 3);
  await p.selectOption("#atkChunk", "1");
  await p.click('[data-attack="silence"]');
  await p.waitForFunction(() => document.querySelectorAll("#chunks li").length === 6 && document.querySelectorAll("#chunks li.bad").length >= 1);
  assert.deepEqual(await p.locator("#chunks li").evaluateAll((els) => els.map((e) => e.className)), ["ok", "bad", "ok", "ok", "ok", "ok"]);
  await c.close();
});

await step("a core that fails to load shows a visible error, never 'Loading core' forever", async () => {
  const c = await browser.newContext();
  const p = await c.newPage();
  await p.route("**/vox_trust.wasm", (route) => route.fulfill({ status: 404, body: "no" }));
  await p.goto(base);
  await p.waitForFunction(() => document.getElementById("coreStatus").classList.contains("err"));
  assert.equal(await p.locator("#coreStatus").innerText(), "Core failed to load");
  assert.equal(await p.locator("#coreStatus").getAttribute("role"), "alert");
  assert.match(await p.locator("#audioInfo").innerText(), /HTTP 404/);
  assert.equal(await p.locator("#sealBtn").isDisabled(), true);
  assert.equal(await p.locator("#useSample").isDisabled(), true);
  await c.close();
});

await step("without JavaScript the noscript note is shown", async () => {
  const c = await browser.newContext({ javaScriptEnabled: false });
  const p = await c.newPage();
  await p.goto(base);
  assert.match(await p.locator("noscript").innerText(), /needs JavaScript and WebAssembly/);
  assert.equal(await p.locator("#coreStatus").isVisible(), false, "no 'Loading core…' chip without JS");
  await c.close();
});

await lctx.close();

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
