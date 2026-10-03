// Shared helpers: the extension API, translations and the stored contacts.
export const api = globalThis.browser ?? globalThis.chrome;

export const t = (key, ...subs) => api.i18n.getMessage(key, subs.map(String)) || key;

export function localize(root) {
  const lang = api.i18n.getUILanguage();
  document.documentElement.lang = lang;
  document.documentElement.dir = lang.startsWith("ar") ? "rtl" : "ltr";
  for (const el of root.querySelectorAll("[data-i18n]")) el.textContent = t(el.dataset.i18n);
  for (const el of root.querySelectorAll("[data-i18n-placeholder]")) el.placeholder = t(el.dataset.i18nPlaceholder);
}

// Contacts live only in this browser (storage.local), never synced.
export async function loadContacts() {
  const { contacts = [] } = await api.storage.local.get("contacts");
  return contacts;
}

export async function saveContacts(contacts) {
  await api.storage.local.set({ contacts });
}

const UNRESERVED = /[A-Za-z0-9\-._~]/;

/**
 * Parses a pairing text (specification, section 9): voxtrust:0:circle|public:<64 hex>[:<label>].
 * Returns { mode, key (hex), label } or null. Strict: the spec allows exactly one text form.
 */
export function parsePairing(text) {
  const m = /^voxtrust:0:(circle|public):([0-9a-f]{64})(?::(.+))?$/.exec(text.trim());
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
    if (new TextEncoder().encode(label).length > 64 || /[\p{Cc}\p{Cf}\p{Zl}\p{Zp}]/u.test(label)) return null;
  }
  return { mode, key, label };
}
