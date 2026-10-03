// The verifier page: verifies a WAV file (dropped, chosen, or from a link) against the
// contacts stored in this browser, with the same WebAssembly core as the demo.
import { loadVoxTrust, hex, unhex } from "./vox-trust.js";
import { api, t, localize, loadContacts, saveContacts, parsePairing } from "./common.js";

const $ = (id) => document.getElementById(id);
localize(document);

const vtReady = loadVoxTrust(api.runtime.getURL("vox_trust.wasm"));
let contacts = await loadContacts();
let current = null; // bytes of the file being verified

function renderContacts() {
  const list = $("contactList");
  list.replaceChildren();
  $("noContacts").hidden = contacts.length > 0;
  contacts.forEach((c, i) => {
    const li = document.createElement("li");
    const who = document.createElement("div");
    who.textContent = c.name;
    const meta = document.createElement("small");
    meta.textContent = `${c.mode} · ${t("keyId", c.keyId)}${c.alwaysSeals ? ` · ${t("alwaysSeals")}` : ""}${c.strict ? ` · ${t("strict")}` : ""}`;
    who.append(meta);
    const remove = document.createElement("button");
    remove.type = "button";
    remove.textContent = t("remove");
    remove.setAttribute("aria-label", `${t("remove")}: ${c.name}`);
    remove.addEventListener("click", async () => {
      contacts.splice(i, 1);
      await saveContacts(contacts);
      renderContacts();
      reverify();
    });
    li.append(who, remove);
    list.append(li);
  });
  const sender = $("sender");
  const keep = sender.value;
  sender.replaceChildren(new Option(t("senderAuto"), ""));
  contacts.forEach((c, i) => sender.append(new Option(c.name, String(i))));
  sender.value = [...sender.options].some((o) => o.value === keep) ? keep : "";
}

async function keyIdOf(mode, keyHex) {
  if (mode === "circle") return (await vtReady).circleKeyId(unhex(keyHex)).toString(16).padStart(8, "0");
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", unhex(keyHex)));
  return hex(digest.slice(0, 4));
}

$("addForm").addEventListener("submit", async (e) => {
  e.preventDefault();
  const parsed = parsePairing($("pairing").value);
  const msg = $("formMsg");
  if (!parsed) {
    msg.textContent = t("invalidPairing");
    return;
  }
  const keyId = await keyIdOf(parsed.mode, parsed.key);
  const name = $("name").value.trim() || parsed.label || t("keyId", keyId);
  contacts = contacts.filter((c) => c.key !== parsed.key);
  contacts.push({ name, mode: parsed.mode, key: parsed.key, keyId, alwaysSeals: $("always").checked, strict: $("strict").checked });
  await saveContacts(contacts);
  msg.textContent = parsed.mode === "circle" ? t("circleWarning") : "";
  e.target.reset();
  renderContacts();
  reverify();
});

function show(kind, title, text, chunks = "", report = null) {
  const box = $("verdict");
  box.hidden = false;
  box.className = `verdict ${kind}`;
  $("vTitle").textContent = title;
  $("vText").textContent = text;
  $("vChunks").textContent = chunks;
  $("vChunks").hidden = !chunks;
  $("report").textContent = report ? JSON.stringify(report, null, 2) : "";
  box.querySelector("details").hidden = !report;
}

// Which contact is the speaker: the one chosen in the menu, else the one whose key sealed it.
function claimedContact(probe) {
  const chosen = $("sender").value;
  if (chosen !== "") return contacts[Number(chosen)];
  if (probe.mode === "public" && probe.embedded_public_key) {
    return contacts.find((c) => c.mode === "public" && c.key === probe.embedded_public_key) ?? null;
  }
  if (probe.mode === "circle" && probe.key_id) {
    return contacts.find((c) => c.mode === "circle" && c.keyId === probe.key_id) ?? null;
  }
  return null;
}

async function verify(bytes) {
  current = bytes;
  const vt = await vtReady;
  let probe;
  try {
    probe = vt.verify(bytes, {});
  } catch {
    show("neutral", t("v_unsealed_title"), t("notWav"));
    return;
  }
  const contact = claimedContact(probe);
  const trust = {};
  if (contact?.mode === "public") trust.pinnedPublicKey = unhex(contact.key);
  if (contact?.mode === "circle") {
    trust.circleKey = unhex(contact.key);
    trust.circleKeyId = parseInt(contact.keyId, 16);
  }
  const report = vt.verify(bytes, trust);
  const verdict = vt.decide(report.check, contact ? { alwaysSeals: contact.alwaysSeals, strict: contact.strict } : null);
  const seconds = report.modified_chunks.map((c) => {
    const from = (c * report.chunk_frames) / report.sample_rate;
    const to = Math.min(((c + 1) * report.chunk_frames) / report.sample_rate, report.n_frames / report.sample_rate);
    return `${from.toFixed(1)}–${to.toFixed(1)} s`;
  });
  const chunks = seconds.length ? t("changedSeconds", seconds.join(", ")) : "";
  const name = contact?.name ?? "";
  let text = t(`v_${verdict}_text`, name);
  if (verdict === "unsealed" && report.check === "unknown_key") {
    text = report.content_matches ? t("unknownKeyIntact") : t("unknownKeyAltered");
  }
  show(verdict === "unsealed" ? "neutral" : verdict, t(`v_${verdict}_title`), text, chunks, report);
}

function reverify() {
  if (current) verify(current);
}

$("sender").addEventListener("change", reverify);
$("file").addEventListener("change", async (e) => {
  const f = e.target.files[0];
  if (f) verify(new Uint8Array(await f.arrayBuffer()));
});
const drop = $("drop");
drop.addEventListener("dragover", (e) => {
  e.preventDefault();
  drop.classList.add("over");
});
drop.addEventListener("dragleave", () => drop.classList.remove("over"));
drop.addEventListener("drop", async (e) => {
  e.preventDefault();
  drop.classList.remove("over");
  const f = e.dataTransfer.files[0];
  if (f) verify(new Uint8Array(await f.arrayBuffer()));
});

// A link from the context menu: ask for access to that one origin, then download it.
async function fromLink(src) {
  let url;
  try {
    url = new URL(src);
  } catch {
    return;
  }
  const box = $("fetchBox");
  box.hidden = false;
  const fetchIt = async () => {
    $("allow").hidden = true;
    $("fetchText").textContent = t("fetching");
    try {
      const res = await fetch(url, { credentials: "include" });
      if (!res.ok) throw new Error(String(res.status));
      verify(new Uint8Array(await res.arrayBuffer()));
      box.hidden = true;
    } catch {
      $("fetchText").textContent = t("fetchFailed");
    }
  };
  if (url.protocol === "data:" || url.protocol === "blob:") {
    $("fetchText").textContent = t("fetchFailed");
    return;
  }
  const origins = [`${url.origin}/*`];
  if (await api.permissions.contains({ origins })) {
    fetchIt();
    return;
  }
  $("fetchText").textContent = t("needAccess", url.origin);
  $("allow").hidden = false;
  $("allow").onclick = async () => {
    if (await api.permissions.request({ origins })) fetchIt();
  };
}

renderContacts();
if (location.hash === "#contacts") $("pairing").focus();
const src = new URLSearchParams(location.search).get("src");
if (src) fromLink(src);
