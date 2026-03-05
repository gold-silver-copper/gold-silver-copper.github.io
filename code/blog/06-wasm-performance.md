# WebAssembly Performance

*2025-06-01*

Compiling Rust to WebAssembly gives near-native performance in
the browser. Grift's arena allocator avoids garbage collection
pauses entirely — memory is managed through a mark-and-sweep
collector that runs on the fixed-size arena. Combined with
Ratzilla's **WebGL2** renderer, the UI maintains smooth 60fps
animation even on mid-range mobile devices.
