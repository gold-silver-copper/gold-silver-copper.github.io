// Service Worker for GRIFT.RS — offline-first caching
// Pre-caches all essential assets on install for full offline support.
// Hashed assets (JS, WASM, CSS) are immutable → cache-first.
// Navigation requests (HTML) → network-first with cache fallback.

var CACHE_NAME = 'grift-v2';

var PRECACHE_URLS = [
  './',
  './index.html',
  './manifest.json',
  './icon-192.png',
  './icon-512.png',
  './fonts/firacode.css',
  './fonts/FiraCode-Regular.woff2',
  './fonts/FiraCode-Bold.woff2',
  './fonts/FiraCode-Light.woff2',
  './fonts/FiraCode-Medium.woff2',
  './fonts/FiraCode-SemiBold.woff2',
  './style-f72f8b68cab0f57c.css',
  './grift-site-f602dd1c3886563b.js',
  './grift-site-f602dd1c3886563b_bg.wasm'
];

self.addEventListener('install', function (event) {
  event.waitUntil(
    caches.open(CACHE_NAME).then(function (cache) {
      return cache.addAll(PRECACHE_URLS);
    }).then(function () {
      return self.skipWaiting();
    })
  );
});

self.addEventListener('activate', function (event) {
  event.waitUntil(
    caches.keys().then(function (keys) {
      return Promise.all(
        keys
          .filter(function (k) { return k !== CACHE_NAME; })
          .map(function (k) { return caches.delete(k); })
      );
    }).then(function () {
      return self.clients.claim();
    })
  );
});

self.addEventListener('fetch', function (event) {
  var request = event.request;

  // Only handle GET requests
  if (request.method !== 'GET') return;

  // Navigation requests (HTML pages): network-first
  if (request.mode === 'navigate') {
    event.respondWith(
      fetch(request)
        .then(function (response) {
          var clone = response.clone();
          caches.open(CACHE_NAME).then(function (cache) {
            cache.put(request, clone);
          });
          return response;
        })
        .catch(function () {
          return caches.match(request);
        })
    );
    return;
  }

  // All other requests (JS, WASM, CSS, fonts, icons): cache-first
  event.respondWith(
    caches.match(request).then(function (cached) {
      if (cached) return cached;
      return fetch(request).then(function (response) {
        if (response.ok) {
          var clone = response.clone();
          caches.open(CACHE_NAME).then(function (cache) {
            cache.put(request, clone);
          });
        }
        return response;
      });
    })
  );
});
