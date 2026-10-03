import { api, localize } from "./common.js";

localize(document);
const open = (hash = "") => {
  api.tabs.create({ url: api.runtime.getURL(`verify.html${hash}`) });
  window.close();
};
document.getElementById("open").addEventListener("click", () => open());
document.getElementById("contacts").addEventListener("click", () => open("#contacts"));
