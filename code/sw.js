// Service Worker for GRIFT.RS — offline-first caching
// Stale-while-revalidate: serve from cache immediately, then update cache
// from network in the background. Navigation requests use network-first.
// Hashed assets (JS, WASM, CSS) are immutable and cached on first fetch.

var CACHE_NAME = 'grift-v4';

var PRECACHE_URLS = [
  './',
  './index.html',
  './manifest.json',
  './icon-192.png',
  './icon-512.png',
  './fonts/jetbrainsmono.css',
  './fonts/JetBrainsMono-Regular.woff2',
  './fonts/JetBrainsMono-Bold.woff2',
  './fonts/JetBrainsMono-Light.woff2',
  './fonts/JetBrainsMono-Medium.woff2',
  './fonts/JetBrainsMono-SemiBold.woff2'
];

self.addEventListener('install', function (event) {
  event.waitUntil(
    caches.open(CACHE_NAME).then(function (cache) {
      // Use individual add() calls so one 404 doesn't block everything
      return Promise.all(
        PRECACHE_URLS.map(function (url) {
          return cache.add(url).catch(function () {
            // Ignore individual failures — the asset may not exist yet
          });
        })
      );
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

  // Navigation requests (HTML pages): network-first with cache fallback
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

  // Hashed assets (contain a hash in filename): cache-first, immutable
  var url = new URL(request.url);
  var isHashedAsset = /\-[a-f0-9]{8,}\.(js|wasm|css)$/.test(url.pathname);

  if (isHashedAsset) {
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
    return;
  }

  // All other assets: stale-while-revalidate
  // Serve from cache immediately, update cache from network in background
  event.respondWith(
    caches.match(request).then(function (cached) {
      var fetchPromise = fetch(request).then(function (response) {
        if (response.ok) {
          var clone = response.clone();
          caches.open(CACHE_NAME).then(function (cache) {
            cache.put(request, clone);
          });
        }
        return response;
      });

      return cached || fetchPromise;
    })
  );
});
