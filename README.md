# gold.silver.copper – Personal Website & Grift Demo

A terminal-themed personal website, blog, project showcase, and interactive REPL for the [Grift](https://github.com/skyfskyf/grift) programming language.

Built entirely with [Ratzilla](https://github.com/ratatui/ratzilla) + [TachyonFX](https://github.com/ratatui/tachyonfx) — terminal UI rendered in the browser via Rust + WebAssembly.

## Features

- **Interactive REPL** — Evaluate Grift expressions directly in the browser
- **Documentation** — Embedded language reference for Grift (basics, forms, advanced)
- **Blog** — Technical articles about Grift, Rust, and terminal UIs
- **Links** — Project repositories and related resources
- **Terminal aesthetic** — Fully rendered as a TUI in the browser
- **TachyonFX effects** — Procedural background animation and page transition effects
- **Clickable navigation** — All tabs, links, and buttons are clickable


## Building

```bash
# Install trunk (WASM bundler)
cargo install trunk

# Add the WASM target
rustup target add wasm32-unknown-unknown

# Serve locally
trunk serve

# Build for production
trunk build --release
```

## Technology

- [Grift](https://github.com/skyfskyf/grift) — A minimalistic Lisp implementing vau calculus (`no_std`, `no_alloc`)
- [Ratzilla](https://github.com/ratatui/ratzilla) — Terminal-themed web apps with Rust + WASM
- [Ratatui](https://github.com/ratatui/ratatui) — Rust TUI framework
- [TachyonFX](https://github.com/ratatui/tachyonfx) — Shader-like effects for terminal UIs

## Navigation

| Action | How |
|--------|-----|
| Switch pages | Click the tabs at the top |
| Evaluate REPL | Type an expression and press Enter |
| Navigate docs | Click ◄ Prev / Next ► buttons or use ←/→ keys |
| Navigate blog | Click a post in the sidebar or use ↑/↓ keys |
| Open links | Click any link on the Links page |
