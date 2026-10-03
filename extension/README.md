# Vox Trust browser extension

Verify sealed voice recordings in Chrome, Edge, Brave, Opera, Firefox and Safari, offline,
with the same WebAssembly core as the [demo](https://vox-trust.github.io/demo/).

- **Verify a file:** drop a WAV file on the verifier page, or right-click a link to an audio
  file (an email attachment, a file in a chat) and choose *Verify with Vox Trust*.
- **Contacts:** paste a contact's pairing text (`voxtrust:0:public:...`); the extension then
  knows their key, and shows **verified**, **warning** or **alert** for their recordings, with
  the seconds that changed.
- **Privacy:** no content scripts, no analytics, nothing uploaded. Contacts are kept in this
  browser only. Reading a link asks for access to that one site, when you click.

**Limits (pre-1.0, not audited):** the seal travels in the file, so it survives only
bit-exact copies. Voice messages that apps re-encode (WhatsApp voice notes, phone calls)
lose it and show "No seal", which is not "fake".

## Install from source

```sh
scripts/build-web.sh          # builds the WebAssembly core
node extension/build.mjs      # writes extension/dist/{chrome,firefox,safari} and zips
```

| Browser | How |
|---|---|
| Chrome, Edge, Brave, Opera | `chrome://extensions` → Developer mode → *Load unpacked* → `extension/dist/chrome` |
| Firefox 128+ | `about:debugging#/runtime/this-firefox` → *Load Temporary Add-on* → `extension/dist/firefox/manifest.json` |
| Safari (macOS) | `xcrun safari-web-extension-converter extension/dist/safari`, build the generated Xcode project, then enable it in Safari → Settings → Extensions (with *Allow unsigned extensions* in the Develop menu) |

Each release also attaches the three zips.

## Store listings

Not published yet. Firefox Add-ons is free; the Chrome Web Store charges a one-time $5;
the Safari App Store needs an Apple Developer account. The zips in `dist/` are what each
store takes.

## Tests

`node tests/extension/extension.mjs` loads the Chrome build in Chromium and checks the four
verdicts, tamper localization, contacts and the link flow. CI also lints the Firefox build
(`web-ext lint`) and converts and builds the Safari version on macOS.
