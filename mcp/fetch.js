// Downloads an audio file for verification without letting a URL reach private networks.
// Every connection checks the address it actually connects to (so DNS rebinding cannot
// slip a private address in after a check), redirects are followed by hand and re-checked,
// and size and time are capped.
import dns from "node:dns";
import http from "node:http";
import https from "node:https";
import net from "node:net";

export const MAX_BYTES = 64 * 1024 * 1024;
const TIMEOUT_MS = 20_000;
const MAX_REDIRECTS = 3;

const blocked = new net.BlockList();
for (const [addr, prefix] of [
  ["0.0.0.0", 8], ["10.0.0.0", 8], ["100.64.0.0", 10], ["127.0.0.0", 8], ["169.254.0.0", 16],
  ["172.16.0.0", 12], ["192.0.0.0", 24], ["192.0.2.0", 24], ["192.168.0.0", 16], ["198.18.0.0", 15],
  ["198.51.100.0", 24], ["203.0.113.0", 24], ["224.0.0.0", 4], ["240.0.0.0", 4],
]) blocked.addSubnet(addr, prefix, "ipv4");
for (const [addr, prefix] of [
  ["::", 128], ["::1", 128], ["64:ff9b::", 96], ["100::", 64], ["2001:db8::", 32],
  ["fc00::", 7], ["fe80::", 10], ["ff00::", 8],
]) blocked.addSubnet(addr, prefix, "ipv6");

export function isPublicAddress(address) {
  const family = net.isIP(address);
  if (family === 0) return false;
  // IPv4-mapped IPv6 (::ffff:a.b.c.d) is judged as the IPv4 address it carries. (BlockList
  // already maps IPv4 onto ::ffff:0:0/96, so that range cannot be listed as blocked.)
  const mapped = /^::ffff:(\d+\.\d+\.\d+\.\d+)$/i.exec(address);
  if (mapped) return isPublicAddress(mapped[1]);
  if (family === 6 && /^::ffff:/i.test(address)) return false;
  return !blocked.check(address, family === 4 ? "ipv4" : "ipv6");
}

function safeLookup(hostname, options, callback) {
  dns.lookup(hostname, { ...options, all: true }, (err, addresses) => {
    if (err) return callback(err);
    const bad = addresses.find((a) => !isPublicAddress(a.address));
    if (bad) return callback(Object.assign(new Error(`refusing to connect to a private address (${bad.address})`), { code: "EPRIVATE" }));
    if (options.all) return callback(null, addresses);
    callback(null, addresses[0].address, addresses[0].family);
  });
}

/** Fetches `url` (https, or http when `allowHttp`) into a Uint8Array. */
export function fetchAudio(url, { allowHttp = false, redirects = 0 } = {}) {
  return new Promise((resolve, reject) => {
    let u;
    try {
      u = new URL(url);
    } catch {
      return reject(new Error("not a valid URL"));
    }
    if (u.protocol !== "https:" && !(allowHttp && u.protocol === "http:")) {
      return reject(new Error(allowHttp ? "only http and https URLs are supported" : "only https URLs are supported"));
    }
    if (u.username || u.password) return reject(new Error("URLs with credentials are not accepted"));
    if (net.isIP(u.hostname.replace(/^\[|\]$/g, "")) && !isPublicAddress(u.hostname.replace(/^\[|\]$/g, ""))) {
      return reject(new Error("refusing to connect to a private address"));
    }
    const lib = u.protocol === "https:" ? https : http;
    const req = lib.get(u, { lookup: safeLookup, timeout: TIMEOUT_MS, headers: { "user-agent": "vox-trust-mcp" } }, (res) => {
      const status = res.statusCode ?? 0;
      if (status >= 300 && status < 400 && res.headers.location) {
        res.resume();
        if (redirects >= MAX_REDIRECTS) return reject(new Error("too many redirects"));
        const next = new URL(res.headers.location, u).toString();
        return fetchAudio(next, { allowHttp, redirects: redirects + 1 }).then(resolve, reject);
      }
      if (status !== 200) {
        res.resume();
        return reject(new Error(`HTTP ${status}`));
      }
      const declared = Number(res.headers["content-length"] ?? 0);
      if (declared > MAX_BYTES) {
        res.destroy();
        return reject(new Error(`file larger than ${MAX_BYTES} bytes`));
      }
      const chunks = [];
      let total = 0;
      res.on("data", (c) => {
        total += c.length;
        if (total > MAX_BYTES) {
          res.destroy();
          reject(new Error(`file larger than ${MAX_BYTES} bytes`));
        } else chunks.push(c);
      });
      res.on("end", () => resolve(new Uint8Array(Buffer.concat(chunks))));
      res.on("error", reject);
    });
    req.on("timeout", () => req.destroy(new Error("download timed out")));
    req.on("error", reject);
  });
}
