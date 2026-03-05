# Building Grift: A Minimalistic Lisp

*2025-02-01*

Grift implements Kernel-style vau calculus with first-class
operatives that subsume both functions and macros.

Key design goals:
- Zero unsafe code (`#![forbid(unsafe_code)]`)
- No heap allocation (arena-only memory)
- Runs on bare-metal embedded systems
- Compiles to WebAssembly

All values live in a fixed-size arena with const-generic
capacity and mark-and-sweep garbage collection.
