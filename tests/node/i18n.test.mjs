// Every demo language must stay in lockstep with English, and the page head must stay correct.
// Run: node --test tests/node/*.mjs
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

import { DEFAULT_LANG, LANGS, LINKS, isolate, langByCode, loadDict, matchLang, pluralCategory, stripControls, translate, translatePlural, user } from "../../web/i18n/index.js";

const read = (path) => readFileSync(new URL(`../../web/${path}`, import.meta.url), "utf8");
const html = read("index.html");
const css = read("demo.css");
const demoJs = read("demo.js");

await Promise.all(LANGS.map((l) => loadDict(l.code)));
const en = langByCode(DEFAULT_LANG).dict;
const others = LANGS.filter((l) => l.code !== DEFAULT_LANG);

const CATEGORIES = ["zero", "one", "two", "few", "many", "other"];
const PLURAL_KEY = new RegExp(`^(.+)_(${CATEGORIES.join("|")})$`);
// A plural message is `base_<category>`; English defines the bases through `base_other`.
const pluralBases = Object.keys(en).filter((k) => k.endsWith("_other")).map((k) => k.slice(0, -"_other".length));
const isPlural = (key) => PLURAL_KEY.test(key) && pluralBases.includes(key.replace(PLURAL_KEY, "$1"));
const plainKeys = (dict) => Object.keys(dict).filter((k) => !isPlural(k)).sort();
// Categories each language must spell out. Others fall back to `_other` (es/pt "many" only occurs for 1,000,000+).
const NEEDED = { en: ["one", "other"], "pt-BR": ["one", "other"], es: ["one", "other"], "zh-Hans": ["other"], ar: ["zero", "one", "two", "few", "many", "other"] };

const placeholders = (text) => [...new Set([...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]))].sort();
// Technical names and units that must survive translation verbatim wherever English uses them.
const KEEP = ["Ed25519", "PBKDF2", "WebAssembly", "ABI", "Apache-2.0", "WAV", "MP3", "M4A", "Rust", "kHz", "MB", "KB", "GitHub", "Vox Trust"];

test("the five supported languages are registered, with ar as the only RTL one", () => {
  assert.deepEqual(LANGS.map((l) => l.code), ["en", "pt-BR", "es", "zh-Hans", "ar"]);
  assert.deepEqual(LANGS.filter((l) => l.dir === "rtl").map((l) => l.code), ["ar"]);
});

test("there is at least one plural message, and English spells out one/other for each", () => {
  assert.ok(pluralBases.length >= 2, pluralBases.join());
  for (const base of pluralBases) for (const c of ["one", "other"]) assert.ok(`${base}_${c}` in en, `${base}_${c}`);
});

for (const lang of others) {
  test(`${lang.code}: same plain keys as en, no empty values, plural categories as the language needs`, () => {
    assert.deepEqual(plainKeys(lang.dict), plainKeys(en));
    for (const [key, value] of Object.entries(lang.dict)) {
      assert.equal(typeof value, "string", key);
      assert.ok(value.trim().length > 0, `${key} is empty`);
    }
    for (const base of pluralBases) {
      const present = CATEGORIES.filter((c) => `${base}_${c}` in lang.dict);
      assert.deepEqual(present, CATEGORIES.filter((c) => NEEDED[lang.code].includes(c)), base);
    }
    for (const key of Object.keys(lang.dict)) if (PLURAL_KEY.test(key)) assert.ok(isPlural(key), `${key} looks plural but English has no such base`);
  });

  test(`${lang.code}: every count resolves to a real message, never to the bare key`, () => {
    for (const base of pluralBases) {
      for (const n of [0, 1, 2, 3, 4, 5, 10, 11, 12, 99, 100, 101, 102, 1000, 1e6, 1.5e6, 1e9]) {
        const text = translatePlural(lang.code, base, n, { mode: "m", chunks: n, bytes: 1, list: "x" });
        assert.notEqual(text, base);
        assert.doesNotMatch(text, /\{\w+\}/, `${base} @ ${n}`);
      }
    }
  });

  test(`${lang.code}: placeholders are a subset of en's, equal for non-plural keys and for _other`, () => {
    for (const [key, value] of Object.entries(lang.dict)) {
      const reference = isPlural(key) ? en[`${key.replace(PLURAL_KEY, "$1")}_other`] : en[key];
      const got = placeholders(value);
      const want = placeholders(reference);
      if (!isPlural(key) || key.endsWith("_other")) assert.deepEqual(got, want, key);
      else for (const p of got) assert.ok(want.includes(p), `${key} uses {${p}}`);
    }
  });

  test(`${lang.code}: markup tokens and protocol names are preserved`, () => {
    for (const [key, value] of Object.entries(en)) {
      const t = lang.dict[key] ?? lang.dict[`${key.replace(PLURAL_KEY, "$1")}_other`];
      assert.ok(t, key);
      assert.equal((t.match(/\*\*/g) || []).length, (value.match(/\*\*/g) || []).length, `${key}: ** count`);
      assert.deepEqual([...t.matchAll(/\]\((\w+)\)/g)].map((m) => m[1]), [...value.matchAll(/\]\((\w+)\)/g)].map((m) => m[1]), `${key}: links`);
      for (const word of KEEP) if (value.includes(word)) assert.ok(t.includes(word), `${key} lost "${word}"`);
    }
  });

  test(`${lang.code}: titles carry their step numeral and the description fits a search snippet`, () => {
    for (const n of [1, 2, 3, 4]) assert.match(lang.dict[`s${n}.title`], new RegExp(`^${n} · `));
    assert.ok(lang.dict["meta.description"].length <= 155, `${lang.dict["meta.description"].length} chars`);
  });
}

test("the English description is at most 155 characters", () => {
  assert.ok(en["meta.description"].length <= 155, `${en["meta.description"].length} chars`);
});

// ---------------------------------------------------------------- Arabic and other plurals

test("Intl.PluralRules drives the category: Arabic has all six, Chinese one, English two", () => {
  const seen = (code) => new Set([0, 1, 2, 3, 10, 11, 99, 100, 101, 102].map((n) => pluralCategory(code, n)));
  assert.deepEqual([...seen("ar")].sort(), ["few", "many", "one", "other", "two", "zero"]);
  assert.deepEqual([...seen("zh-Hans")], ["other"]);
  assert.deepEqual([...seen("en")].sort(), ["one", "other"]);
});

test("Arabic picks the right wording for 0, 1, 2, 3-10, 11-99 and 100", () => {
  const say = (n) => translatePlural("ar", "seal.done", n, { mode: "x", chunks: n, bytes: 5 });
  assert.match(say(0), /بلا مقاطع/);
  assert.match(say(1), /مقطع واحد/);
  assert.match(say(2), /مقطعان/);
  assert.match(say(3), /3.* مقاطع،/);
  assert.match(say(10), /10.* مقاطع،/);
  assert.match(say(11), /11.* مقطعًا،/);
  assert.match(say(99), /99.* مقطعًا،/);
  assert.match(say(100), /100.* مقطع،/);
  assert.match(translatePlural("ar", "v.alert.modified.text", 2, { list: "1, 2" }), /المقطعان المعدَّلان/);
  assert.match(translatePlural("ar", "v.alert.modified.text", 1, { list: "1" }), /المقطع المعدَّل:/);
});

test("English and Portuguese use one/other, Chinese always other", () => {
  assert.match(translatePlural("en", "seal.done", 1, { mode: "m", chunks: 1, bytes: 1 }), /1 chunk,/);
  assert.match(translatePlural("en", "seal.done", 0, { mode: "m", chunks: 0, bytes: 1 }), /0 chunks,/);
  assert.match(translatePlural("pt-BR", "seal.done", 2, { mode: "m", chunks: 2, bytes: 1 }), /2 trechos/);
  assert.match(translatePlural("zh-Hans", "seal.done", 1, { mode: "m", chunks: 1, bytes: 1 }), /1 个片段/);
});

// ---------------------------------------------------------------- the runtime

test("matchLang maps browser tags to supported codes", () => {
  assert.equal(matchLang("pt"), "pt-BR");
  assert.equal(matchLang("pt-PT"), "pt-BR");
  assert.equal(matchLang("es-MX"), "es");
  assert.equal(matchLang("zh-CN"), "zh-Hans");
  assert.equal(matchLang("zh-Hans-SG"), "zh-Hans");
  assert.equal(matchLang("zh-TW"), null);
  assert.equal(matchLang("zh-Hant"), null);
  assert.equal(matchLang("AR-eg"), "ar");
  assert.equal(matchLang("fr"), null);
  assert.equal(matchLang(null), null);
});

test("translate substitutes params, falls back to en, and isolates numbers in RTL", () => {
  assert.equal(translate("en", "core.ready", { abi: 1 }), "Core ready · ABI 1");
  assert.equal(translate("es", "no.such.key"), "no.such.key");
  assert.equal(translate("xx", "v.verified.title"), "Verified");
  assert.match(translate("ar", "core.ready", { abi: 1 }), /⁨1⁩/);
  assert.doesNotMatch(translate("es", "core.ready", { abi: 1 }), /⁨/);
  assert.equal(translate("en", "pin.invalid", {}), en["pin.invalid"]); // missing params stay visible
});

test("translate never reads prototype keys, for messages or for params", () => {
  for (const key of ["constructor", "__proto__", "toString", "hasOwnProperty", "valueOf"]) {
    assert.equal(translate("en", key), key);
    assert.equal(translate("ar", key), key);
  }
  assert.equal(translate("en", "pin.invalid", Object.create({ n: "inherited" })), en["pin.invalid"]);
  assert.equal(translatePlural("en", "constructor", 1), "constructor");
});

test("user-supplied params are always cleaned and isolated, in every language, and never re-scanned", () => {
  const hostile = "a‮b{abi}**c​\n";
  for (const code of ["en", "es", "zh-Hans", "ar"]) {
    const text = translate(code, "audio.info", { label: user(hostile), seconds: 1, khz: 16, kb: 2 });
    assert.ok(text.includes("⁨ab{abi}**c⁩"), `${code}: ${JSON.stringify(text)}`);
    assert.doesNotMatch(text, /[‮​\n]/);
  }
  assert.equal(stripControls("x‮y\u0000z‏w"), "xyzw");
  assert.equal(isolate("a‮b"), "⁨ab⁩");
  assert.equal(translate("en", "audio.info", { label: "plain", seconds: 1, khz: 16, kb: 2 }), "plain: 1 s, 16 kHz mono, 2 KB.");
});

// ---------------------------------------------------------------- markup vs dictionaries

const decode = (s) => s.replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&quot;/g, '"').replace(/&#39;/g, "'").replace(/&amp;/g, "&");
const squash = (s) => s.replace(/\s+/g, " ").trim();

// Elements carrying `attr="key"` with their raw inner HTML (children included).
function withAttr(attr) {
  const found = [];
  for (const open of html.matchAll(new RegExp(`<([a-z0-9]+)\\b([^>]*?\\b${attr}="([^"]+)"[^>]*)>`, "g"))) {
    const [whole, tag, attrs, key] = open;
    let depth = 1;
    const re = new RegExp(`<(/?)${tag}\\b[^>]*>`, "g");
    re.lastIndex = open.index + whole.length;
    let inner = null;
    for (let m = re.exec(html); m; m = re.exec(html)) {
      depth += m[1] ? -1 : 1;
      if (depth === 0) {
        inner = html.slice(open.index + whole.length, m.index);
        break;
      }
    }
    found.push({ key, tag, attrs, inner });
  }
  return found;
}

const linkIdFor = (href) => Object.entries(LINKS).find(([, l]) => l.href === decode(href))?.[0] ?? `UNKNOWN(${href})`;
const toText = (inner) => squash(decode(inner.replace(/<[^>]+>/g, "")));
const toRich = (inner) =>
  squash(
    decode(
      inner
        .replace(/<strong>([\s\S]*?)<\/strong>/g, "**$1**")
        .replace(/<a\b[^>]*\bhref="([^"]+)"[^>]*>([\s\S]*?)<\/a>/g, (_, href, text) => `[${text}](${linkIdFor(href)})`)
        .replace(/<[^>]+>/g, ""),
    ),
  );

test("every data-i18n* key used in index.html exists in en", () => {
  const used = [...html.matchAll(/data-i18n(?:-[a-z]+)?="([^"]+)"/g)].map((m) => m[1]);
  assert.ok(used.length > 40);
  for (const key of used) assert.ok(Object.hasOwn(en, key), `index.html uses unknown key ${key}`);
});

test("the English fallback in index.html matches en.js: plain, rich (with children), aria, placeholder, content", () => {
  const plain = withAttr("data-i18n");
  const rich = withAttr("data-i18n-rich");
  assert.ok(plain.length > 30 && rich.length >= 5, `${plain.length} plain, ${rich.length} rich`);
  for (const { key, inner } of plain) assert.equal(toText(inner), squash(en[key]), key);
  for (const { key, inner } of rich) assert.equal(toRich(inner), squash(en[key]), key);
  for (const [attr, target] of [["data-i18n-aria", "aria-label"], ["data-i18n-placeholder", "placeholder"], ["data-i18n-content", "content"]]) {
    const items = withAttr(attr);
    assert.ok(items.length >= 1, attr);
    for (const { key, attrs } of items) assert.equal(decode(attrs.match(new RegExp(`(?<![\\w-])${target}="([^"]*)"`))[1]), en[key], `${attr} ${key}`);
  }
});

test("every [text](id) in every dictionary names a link that exists, with a safe https target", () => {
  const ids = new Set();
  for (const l of LANGS) for (const value of Object.values(l.dict)) for (const m of value.matchAll(/\]\((\w+)\)/g)) ids.add(m[1]);
  assert.ok(ids.size >= 2);
  for (const id of ids) assert.ok(Object.hasOwn(LINKS, id), `link id "${id}" is not in LINKS`);
  for (const [id, link] of Object.entries(LINKS)) assert.match(link.href, /^https:\/\//, id);
});

test("every i18n key the demo code uses exists (dynamic prefixes enumerated)", () => {
  const codes = [...new Set([...demoJs.matchAll(/coded\("(\w+)"/g), ...read("vox-trust.js").matchAll(/coded\("(\w+)"/g)].map((m) => m[1]))];
  const dynamic = { "mode.name.": ["circle", "public"], "err.": codes };
  const keys = new Set();
  for (const re of [/\b(?:t|msg)\(\s*"([^"]+)"/g, /\bk:\s*"([^"]+)"/g, /\b(?:title|text):\s*"([^"]+)"/g]) for (const m of demoJs.matchAll(re)) keys.add(m[1]);
  for (const m of demoJs.matchAll(/`([^`$]*)\$\{[^`]*`/g)) {
    if (!/^[a-z]+\.[\w.]*$/.test(m[1])) continue;
    assert.ok(dynamic[m[1]], `demo.js builds keys from the unknown prefix "${m[1]}"; enumerate it in this test`);
    for (const v of dynamic[m[1]]) keys.add(m[1] + v);
  }
  assert.ok(keys.size > 40, `only ${keys.size} keys found`);
  assert.ok(codes.length >= 5);
  for (const key of keys) assert.ok(Object.hasOwn(en, key) || Object.hasOwn(en, `${key}_other`), `demo.js uses missing key ${key}`);
});

test("the markup carries no decorative icon, step badge or waveform any more", () => {
  assert.doesNotMatch(html, /<canvas|vIcon|v-icon|class="step"|<img|<svg/);
  assert.doesNotMatch(css, /\.wave|\.v-icon|\.step\b|--wave/);
  assert.doesNotMatch(demoJs, /drawWave|getContext|vIcon/);
  assert.match(html, /id="audioInfo"/);
});

// ---------------------------------------------------------------- SEO head

const meta = (attr, name) => html.match(new RegExp(`<meta ${attr}="${name}"[^>]*? content="([^"]*)"`))?.[1];

test("head: title, description, canonical, robots, theme colours", () => {
  assert.equal(decode(html.match(/<title[^>]*>([^<]+)<\/title>/)[1]), "Vox Trust demo: seal a voice, try to cheat it, verify in your browser");
  assert.equal(decode(html.match(/<title[^>]*>([^<]+)<\/title>/)[1]), en["meta.title"]);
  const description = decode(meta("name", "description"));
  assert.ok(description.length <= 155 && description.length >= 80, `${description.length}`);
  assert.equal(description, en["meta.description"]);
  assert.match(html, /<link rel="canonical" href="https:\/\/vox-trust\.github\.io\/demo\/">/);
  assert.equal(html.match(/rel="canonical"/g).length, 1);
  assert.equal(meta("name", "robots"), "index,follow");
  assert.doesNotMatch(html, /hreflang/, "?lang variants are client-side only, so the demo has no hreflang");
  assert.match(html, /<meta name="theme-color" media="\(prefers-color-scheme: light\)" content="#fbfbf9">/);
  assert.match(html, /<meta name="theme-color" media="\(prefers-color-scheme: dark\)" content="#0e1013">/);
  assert.match(html, /<html lang="en" dir="ltr">/);
});

test("head: Open Graph and Twitter card", () => {
  assert.equal(meta("property", "og:type"), "website");
  assert.equal(decode(meta("property", "og:title")), en["meta.title"]);
  assert.equal(decode(meta("property", "og:description")), en["meta.description"]);
  assert.equal(meta("property", "og:url"), "https://vox-trust.github.io/demo/");
  assert.equal(meta("property", "og:image"), "https://vox-trust.github.io/og.png");
  assert.equal(meta("property", "og:image:width"), "1200");
  assert.equal(meta("property", "og:image:height"), "630");
  assert.ok(meta("property", "og:image:alt").length > 10);
  assert.equal(meta("property", "og:site_name"), "Vox Trust");
  assert.equal(meta("name", "twitter:card"), "summary_large_image");
});

test("head: favicons use absolute paths", () => {
  assert.match(html, /<link rel="icon" href="\/favicon\.svg" type="image\/svg\+xml" sizes="any">/);
  assert.match(html, /<link rel="icon" href="\/favicon\.ico" sizes="32x32">/);
  assert.match(html, /<link rel="apple-touch-icon" href="\/apple-touch-icon\.png">/);
});

test("head: JSON-LD is a valid WebApplication and the CSP needs no unsafe-inline for it", () => {
  const block = html.match(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/);
  assert.ok(block, "JSON-LD script is present");
  const ld = JSON.parse(block[1]);
  assert.equal(ld["@context"], "https://schema.org");
  assert.equal(ld["@type"], "WebApplication");
  assert.equal(ld.url, "https://vox-trust.github.io/demo/");
  assert.equal(ld.applicationCategory, "SecurityApplication");
  assert.equal(ld.operatingSystem, "Any (modern browser)");
  assert.equal(ld.browserRequirements, "Requires WebAssembly");
  assert.equal(ld.isAccessibleForFree, true);
  assert.deepEqual(ld.offers, { "@type": "Offer", "price": "0", "priceCurrency": "USD" });
  assert.deepEqual(ld.inLanguage, ["en", "pt-BR", "es", "zh-Hans", "ar"]);
  assert.equal(ld.isPartOf["@type"], "WebSite");
  assert.equal(ld.isPartOf.url, "https://vox-trust.github.io/");
  assert.ok(ld.name && ld.description);
  const csp = html.match(/Content-Security-Policy" content="([^"]+)"/)[1];
  assert.match(csp, /script-src 'self' 'wasm-unsafe-eval'(;|$)/);
  assert.doesNotMatch(csp, /unsafe-inline/);
  // The only other script is the module; no inline code runs.
  const scripts = [...html.matchAll(/<script\b([^>]*)>/g)].map((m) => m[1]);
  assert.deepEqual(scripts.filter((a) => !a.includes("ld+json")), [' type="module" src="demo.js"']);
});

test("a noscript fallback explains the requirement and links to the overview and the source", () => {
  const noscript = html.match(/<noscript>([\s\S]*?)<\/noscript>/)[1];
  assert.match(noscript, /JavaScript/);
  assert.match(noscript, /href="https:\/\/vox-trust\.github\.io\/"/);
  assert.match(noscript, /href="https:\/\/github\.com\/vox-trust\/vox-trust"/);
});

// ---------------------------------------------------------------- CSS tokens

const luminance = (hex) => {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255).map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
};
const contrast = (a, b) => {
  const [x, y] = [luminance(a), luminance(b)];
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
};
const tokens = (block) => Object.fromEntries([...block.matchAll(/--([\w-]+):(#[0-9a-f]{6})/gi)].map((m) => [m[1], m[2]]));

test("CSS: color-scheme is declared and control borders reach 3:1 against bg and card in both themes", () => {
  assert.match(css, /:root\{\s*color-scheme:light dark;/);
  const light = tokens(css.slice(0, css.indexOf("@media (prefers-color-scheme:dark)")));
  const dark = tokens(css.slice(css.indexOf("@media (prefers-color-scheme:dark)")));
  for (const [name, theme] of [["light", light], ["dark", dark]]) {
    assert.ok(theme["control-border"], `${name} --control-border`);
    for (const surface of ["bg", "card"]) assert.ok(contrast(theme["control-border"], theme[surface]) >= 3, `${name} control-border vs ${surface}: ${contrast(theme["control-border"], theme[surface]).toFixed(2)}`);
    assert.notEqual(theme["control-border"], theme.border);
  }
  for (const selector of ["input[type=text],input[type=password],select", "fieldset", "button,.btn"]) {
    const rule = css.split("\n").find((line) => line.startsWith(selector));
    assert.ok(rule && rule.includes("var(--control-border)"), selector);
  }
});
