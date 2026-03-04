# TachyonFX: Shader Effects for TUIs

*2025-07-10*

TachyonFX brings shader-like visual effects to terminal UIs.
Effects like fade, sweep, slide, coalesce, and HSL shift can
be composed with combinators like `ping_pong` and `repeating`.
Each effect operates on a rectangular cell region and tracks
its own timing via `EffectTimer`. The library integrates with
Ratatui's rendering pipeline through the `EffectRenderer` trait,
making it easy to add polish to any terminal application.
