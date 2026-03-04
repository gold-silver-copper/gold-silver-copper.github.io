# Unified Layout Design

*2025-05-15*

Traditional responsive design uses breakpoints to switch between
mobile and desktop layouts. This site takes a different approach:
there is only one layout that works everywhere. The terminal grid
scales naturally to any screen size, and touch gestures work
alongside mouse and keyboard input. No media queries, no
breakpoints, no separate code paths. The same Rust code renders
identically on a phone, tablet, or ultrawide monitor.
