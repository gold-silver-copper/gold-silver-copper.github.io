# Arena Allocation in Grift

*2025-08-05*

Grift uses a fixed-size arena for all allocations. The arena
is a contiguous array of cells, each holding a Lisp value.
Const generics set the capacity at compile time — no runtime
overhead, no dynamic allocation, no unsafe code. A mark-and-sweep
garbage collector reclaims unreachable cells. This design makes
Grift suitable for embedded systems with no heap and for WASM
targets where memory management must be predictable.
