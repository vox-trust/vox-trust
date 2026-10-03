// The verification logic behind the MCP tools, kept apart from the transport so it can be
// tested directly. Uses the same WebAssembly core and trust policy as the demo.
import { createHash } from "node:crypto";
import { readFile, stat } from "node:fs/promises";
import { loadVoxTrust, hex, unhex } from "./core/vox-trust.js";
import { fetchAudio, MAX_BYTES } from "./fetch.js";

const vtReady = readFile(new URL("./core/vox_trust.wasm", import.meta.url)).then((b) => loadVoxTrust(b));

const UNRESERVED = /[A-Za-z0-9\-._~]/;

/** Parses a pairing text (specification, section 9). Returns null if it is not valid. */
export function parsePairing(text) {
  const m = /^voxtrust:0:(circle|public):([0-9a-f]{64})(?::(.+))?$/.exec(String(text).trim());
  if (!m) return null;
  const [, mode, key, raw] = m;
  let label = null;
  if (raw !== undefined) {
    if (!/^(?:[A-Za-z0-9\-._~]|%[0-9A-F]{2})+$/.test(raw)) return null;
    for (const esc of raw.matchAll(/%([0-9A-F]{2})/g)) {
      if (UNRESERVED.test(String.fromCharCode(parseInt(esc[1], 16)))) return null;
    }
    try {
      label = decodeURIComponent(raw);
    } catch {
      return null;
    }
    if (Buffer.byteLength(label) > 64 || /[\p{Cc}\p{Cf}\p{Zl}\p{Zp}]/u.test(label)) return null;
  }
  return { mode, key, label };
}

export const publicKeyId = (keyHex) => createHash("sha256").update(Buffer.from(keyHex, "hex")).digest("hex").slice(0, 8);

/** A public key given as 64 hex characters or as a public pairing text. */
export function readPublicKey(input) {
  const s = String(input).trim();
  if (/^[0-9a-fA-F]{64}$/.test(s)) return { key: s.toLowerCase(), label: null };
  const p = parsePairing(s);
  if (!p) throw new Error("pinned_public_key must be 64 hex characters or a voxtrust:0:public:... pairing text");
  if (p.mode !== "public") throw new Error("this is a circle pairing text, which contains a shared secret; never send it to a server. Use a public key.");
  return { key: p.key, label: p.label };
}

async function loadAudio({ url, base64, path }, { remote }) {
  const given = [url, base64, path].filter((v) => v !== undefined && v !== "");
  if (given.length !== 1) throw new Error("give exactly one of url, base64 or path");
  if (base64) {
    const bytes = Buffer.from(base64, "base64");
    if (bytes.length > MAX_BYTES) throw new Error(`file larger than ${MAX_BYTES} bytes`);
    return new Uint8Array(bytes);
  }
  if (url) return fetchAudio(url, { allowHttp: !remote });
  if (remote) throw new Error("local paths are not available on the hosted server; pass a url or base64");
  const info = await stat(path);
  if (!info.isFile()) throw new Error("path is not a regular file");
  if (info.size > MAX_BYTES) throw new Error(`file larger than ${MAX_BYTES} bytes`);
  return new Uint8Array(await readFile(path));
}

const EXPLAIN = {
  verified: "Valid seal from the pinned key, and the audio has not changed since it was sealed.",
  unsealed: "No seal that can be checked. Normal for most audio; it does not mean the audio is fake.",
  warning: "This contact always seals, but the recording has no valid seal from them. Be careful.",
  alert: "The seal is broken, the audio was altered after sealing, or it was sealed by a different key than the pinned one. Do not trust it.",
};

/** Verifies a WAV file. Returns a plain object meant for both people and programs. */
export async function verifyAudio(args, { remote = false } = {}) {
  const vt = await vtReady;
  const bytes = await loadAudio(args, { remote });
  const pinned = args.pinned_public_key ? readPublicKey(args.pinned_public_key) : null;
  let report;
  try {
    report = vt.verify(bytes, pinned ? { pinnedPublicKey: unhex(pinned.key) } : {});
  } catch (e) {
    return {
      verdict: "unsealed",
      explanation: `${EXPLAIN.unsealed} The file could not be read as a 16-bit PCM WAV file (${e.message}). Voice messages that apps re-encode lose the seal.`,
      check: "absent",
      reason: "not_a_wav_file",
    };
  }
  // Pinning a key means the speaker uses Vox Trust: a missing seal or another key then matters.
  const contact = pinned ? { alwaysSeals: args.always_seals !== false || Boolean(args.strict), strict: Boolean(args.strict) } : null;
  const verdict = vt.decide(report.check, contact);
  const changed = report.modified_chunks.map((c) => ({
    chunk: c,
    from_seconds: +((c * report.chunk_frames) / report.sample_rate).toFixed(3),
    to_seconds: +(Math.min((c + 1) * report.chunk_frames, report.n_frames) / report.sample_rate).toFixed(3),
  }));
  let explanation = EXPLAIN[verdict];
  if (report.check === "unknown_key") {
    explanation += report.content_matches
      ? " It carries a valid seal from a key that was not pinned, and the audio is unchanged since sealing; that says nothing about who sealed it."
      : " It carries a seal from a key that was not pinned, and the audio was altered after sealing.";
  }
  if (!pinned && report.check !== "absent") {
    explanation += " Pass pinned_public_key to check it against a specific person.";
  }
  return {
    verdict,
    explanation,
    check: report.check,
    reason: report.reason,
    mode: report.mode,
    key_id: report.key_id,
    sealed_by_public_key: report.authenticator_valid ? report.embedded_public_key : null,
    pinned_key_id: pinned ? publicKeyId(pinned.key) : null,
    created_unix: report.authenticated ? report.created_unix : null,
    audio_unchanged_since_sealing: report.content_matches,
    changed_seconds: changed,
    duration_seconds: +(report.n_frames / report.sample_rate).toFixed(3),
    sample_rate: report.sample_rate,
    channels: report.channels,
  };
}

/** Describes a pairing text without keeping it. */
export function describePairing(text) {
  const p = parsePairing(text);
  if (!p) return { valid: false, explanation: "Not a valid pairing text (voxtrust:0:circle|public:<64 lowercase hex>[:<label>])." };
  if (p.mode === "circle") {
    return {
      valid: true,
      mode: "circle",
      label: p.label,
      explanation: "A circle pairing text contains a secret shared by a group. Keep it private and never paste it into online services; verify circle recordings locally (the browser extension, the demo or the command line).",
    };
  }
  return { valid: true, mode: "public", public_key: p.key, key_id: publicKeyId(p.key), label: p.label, explanation: "A public key. Use it as pinned_public_key in verify_audio." };
}

export { hex };
