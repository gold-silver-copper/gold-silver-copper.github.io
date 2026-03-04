# Ideas — Features Begging to Be Added

These things should certainly be part of the website but aren't yet. They are just begging to be added.

---

**Command palette / search bar.**
The terminal aesthetic is already there — a `/` or `:` command input that searches blog posts,
navigates to tabs, or runs Grift expressions inline would feel completely natural. The REPL
input infrastructure already exists; a command palette is just a different dispatch on Enter.

**Persistent REPL history across sessions.**
The Grift REPL loses everything on page reload. Saving history to `localStorage` (serialized
as JSON) would make the REPL actually useful for iterating on expressions. The `repl_history`
vec just needs a load/save cycle at init and on each evaluation.

**Syntax highlighting in the REPL.**
Right now REPL input and output are monochrome. Grift's S-expressions have clear structural
tokens — parentheses, symbols, numbers, strings, keywords — that could be colored inline
using ratatui's `Span` styling. The tokenizer already exists in Grift; it just needs a
display-side pass.

**Shareable REPL sessions via URL hash.**
Encode the current REPL input (or a sequence of expressions) into the URL fragment so that
`grift.rs/#(define x 42)` pre-loads and evaluates an expression on page load. This turns
the site into a linkable playground.

**Blog post Markdown rendering.**
Blog content is currently plain `&str` tuples. Supporting a minimal Markdown subset
(headers, bold, italic, code blocks, links) inside the blog entries would make posts
more readable without adding a full Markdown parser — just a small pass that maps
patterns to ratatui `Style` and `Span` values.

**Keyboard shortcut cheat sheet overlay.**
A `?` key or dedicated help tab that shows all keybindings in a styled overlay.
The focus/unfocus system, Ctrl+B/F in REPL, virtual keyboard layout — users shouldn't
have to guess. A single reference panel would make the site approachable.

**Theme switcher.**
The dark blue/copper palette is great, but a toggle between a few color themes (solarized,
gruvbox, dracula, light mode) would be trivial since all colors are already `Color::Rgb`
constants. Pull them into a `Theme` struct and let users pick.

**Animated ASCII art header.**
The Home page has text descriptions but no visual flair at the top. A small ASCII art logo
or animated banner using TachyonFX effects would set the tone immediately and use the
visual tooling that's already integrated.

**RSS/Atom feed generation.**
`BLOG_ENTRIES` is a static slice — a build-time script (or a dedicated Rust binary) could
generate an XML feed from it. Blog-style sites without feeds are missing a basic distribution
channel.

**Copy-to-clipboard button on REPL output.**
A small `[⧉]` button next to each REPL result that copies the output text. The
paste-support infrastructure (clipboard API) is already wired; copying is the mirror
operation.

**Link previews on the About page.**
Each link already has a description string. Rendering a tooltip or inline preview paragraph
on hover/focus (instead of just the URL) would surface that information without requiring
a click.

**Tab reordering / pinning.**
Let users drag tabs or use a shortcut to reorder them. Power users visiting frequently
may want REPL first and Home last. The tab system is already a carousel — making the
order user-configurable is a natural extension.

**Offline PWA support.**
The site already has a manifest-ready structure (fixed viewport, theme-color, standalone
capable). Adding a Service Worker that caches the WASM binary and static assets would
make it work fully offline — perfect for the "zero dependencies" philosophy.

**Touch gesture for tab switching.**
Horizontal swipes in the content area currently dispatch a single ArrowLeft/ArrowRight.
A more fluid gesture — swipe with momentum that animates the tab carousel — would feel
native on mobile and take advantage of the existing carousel transition effects.

**Embedded Grift examples on the Home and Docs pages.**
Clickable code snippets that, when tapped, auto-navigate to the REPL tab and pre-fill
the input. The infrastructure is there (switch_page + setting repl_input); the missing
piece is wiring it to clickable code blocks in other tabs.
