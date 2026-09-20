// Fold service worker. App shell: network first (so a deploy shows up on the next load), cache as the offline fallback.
// Pack files: cache first — they are immutable for a given build, and the cache name changes with every build.
const CACHE = "fold-__VERSION__";
self.addEventListener("install", e => { self.skipWaiting(); e.waitUntil(caches.open(CACHE).then(c => c.addAll(["./", "manifest.webmanifest", "icon-192.png"]))); });
self.addEventListener("activate", e => e.waitUntil(caches.keys().then(ks => Promise.all(ks.filter(k => k !== CACHE).map(k => caches.delete(k)))).then(() => self.clients.claim())));
self.addEventListener("fetch", e => {
  const u = new URL(e.request.url);
  if (e.request.method !== "GET" || u.origin !== location.origin) return;
  const put = r => { if (r.ok) { const copy = r.clone(); caches.open(CACHE).then(c => c.put(e.request, copy)); } return r; };
  if (u.pathname.includes("/pack/")) e.respondWith(caches.match(e.request).then(hit => hit || fetch(e.request).then(put)));
  else e.respondWith(fetch(e.request).then(put).catch(() => caches.match(e.request, { ignoreSearch: true }).then(hit => hit || caches.match("./"))));
});
