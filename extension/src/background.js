// Adds "Verify with Vox Trust" to the context menu of links and audio elements. The click
// opens the verifier page with the address; nothing is fetched until the page asks.
const api = globalThis.browser ?? globalThis.chrome;

api.runtime.onInstalled.addListener(() => {
  api.contextMenus.create({
    id: "vox-trust-verify",
    title: api.i18n.getMessage("menuVerify"),
    contexts: ["link", "audio"],
  });
});

api.contextMenus.onClicked.addListener((info) => {
  const src = info.srcUrl || info.linkUrl;
  if (!src) return;
  api.tabs.create({ url: api.runtime.getURL(`verify.html?src=${encodeURIComponent(src)}`) });
});
