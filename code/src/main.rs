use std::cell::RefCell;
use std::rc::Rc;

use grift::Lisp;
use ratzilla::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratzilla::ratatui::layout::{Alignment, Constraint, Layout, Offset, Position, Rect};
use ratzilla::ratatui::style::{Color, Modifier, Style, Stylize};
use ratzilla::ratatui::text::{Line, Span, Text};
use ratzilla::ratatui::widgets::{Block, BorderType, List, ListItem, Paragraph, Wrap};
use ratzilla::ratatui::Frame;
use ratzilla::backend::webgl2::WebGl2BackendOptions;
use ratzilla::WebGl2Backend;
use ratzilla::WebRenderer;

use tachyonfx::fx::{self};
use tachyonfx::{CellFilter, Duration, Effect, EffectRenderer, EffectTimer, Interpolation, Motion};

const TRAIL_INITIAL_INTENSITY: u8 = 200;
const MAX_TRAIL_LENGTH: usize = 30;
const TRAIL_FADE_RATE: u8 = 8;
const CURSOR_BLINK_RATE: u64 = 60;

// Responsive layout breakpoints (in terminal grid columns/rows)
const NARROW_WIDTH_THRESHOLD: u16 = 50;
const VERY_NARROW_WIDTH_THRESHOLD: u16 = 35;
const NARROW_MARGIN_THRESHOLD: u16 = 60;
const SHORT_MARGIN_THRESHOLD: u16 = 30;

const BANNER: &str = r#"
  ██████╗ ██████╗ ██╗███████╗████████╗   ██████╗ ███████╗
 ██╔════╝ ██╔══██╗██║██╔════╝╚══██╔══╝   ██╔══██╗██╔════╝
 ██║  ███╗██████╔╝██║█████╗     ██║      ██████╔╝███████╗
 ██║   ██║██╔══██╗██║██╔══╝     ██║      ██╔══██╗╚════██║
 ╚██████╔╝██║  ██║██║██║        ██║   ██╗██║  ██║███████║
  ╚═════╝ ╚═╝  ╚═╝╚═╝╚═╝        ╚═╝   ╚═╝╚═╝  ╚═╝╚══════╝
"#;

const DESCRIPTION: &str = "\
Grift is a minimalistic Lisp implementing Kernel-style vau calculus.\n\
\n\
Features:\n\
  • First-class operatives (fexprs) that subsume functions and macros\n\
  • no_std, no_alloc — runs on bare-metal embedded systems\n\
  • Arena-allocated with const-generic capacity\n\
  • Tail-call optimized with mark-and-sweep GC\n\
  • Zero unsafe code (#![forbid(unsafe_code)])\n\
  • Compiles to WebAssembly";

const ABOUT: &str = "\
>_ gold.silver.copper — Software developer • Rust enthusiast • Language designer\n\
Interests: programming languages, NLP, AI, game dev, web dev.";

const LISP_INFO: &str = "\
Lisp is one of the oldest and most influential programming languages.\n\
Its uniform syntax (S-expressions) and homoiconicity make it uniquely\n\
suited to metaprogramming and language research. Code is data, data\n\
is code — enabling macros, DSLs, and self-modifying programs.";

const VAU_INFO: &str = "\
Vau calculus extends the lambda calculus with first-class operatives\n\
(fexprs). Unlike macros, operatives receive their arguments\n\
unevaluated and can inspect the caller's environment. This makes\n\
them strictly more powerful — subsuming both functions and macros\n\
in a single, elegant primitive.";

const RUST_INFO: &str = "\
Rust is a systems programming language focused on safety, speed,\n\
and concurrency. Grift is written in pure Rust with no_std,\n\
no_alloc — it compiles to WebAssembly and runs on bare metal.\n\
This entire website is Rust, rendered as a terminal UI via WASM.";

const GSC_INFO: &str = "\
gold.silver.copper is a software developer passionate about\n\
programming languages and systems programming. Grift is their\n\
minimalistic Lisp built on vau calculus — featuring arena\n\
allocation, tail-call optimization, and mark-and-sweep GC,\n\
all in safe Rust with zero unsafe code.";

const LINKS: &[(&str, &str)] = &[
    (
        "GitHub (gold-silver-copper)",
        "https://github.com/gold-silver-copper",
    ),
    ("GitHub (grift)", "https://github.com/skyfskyf/grift"),
    (
        "GitHub (grift-site)",
        "https://github.com/skyfskyf/grift-site",
    ),
    ("Ratzilla – Terminal web apps with Rust + WASM", "https://github.com/ratatui/ratzilla"),
    ("Ratatui – Terminal UI framework", "https://github.com/ratatui/ratatui"),
    ("TachyonFX – Shader-like effects for TUIs", "https://github.com/ratatui/tachyonfx"),
    ("WebAssembly", "https://webassembly.org"),
];

const DOC_BASICS: &str = "\
Grift Basics\n\
────────────\n\
\n\
Grift is a Kernel-style Lisp with first-class operatives (fexprs).\n\
All values live in a fixed-size arena with const-generic capacity.\n\
\n\
Atoms:\n\
  42          ; number\n\
  #t #f       ; booleans\n\
  hello       ; symbol\n\
  \"hello\"     ; string\n\
  ()          ; nil / empty list\n\
  #inert      ; inert value (side-effect returns)\n\
  #ignore     ; ignore (parameter matching)\n\
\n\
Arithmetic:\n\
  (+ 1 2)           => 3\n\
  (* 6 7)           => 42\n\
  (- 10 3)          => 7\n\
  (/ 20 4)          => 5\n\
  (mod 10 3)        => 1\n\
\n\
Comparison:\n\
  (=? 1 1)          => #t\n\
  (<? 1 2)          => #t\n\
  (>? 2 1)          => #t\n\
  (<=? 1 1)         => #t\n\
  (>=? 2 1)         => #t";

const DOC_FORMS: &str = "\
Special Forms & Definitions\n\
───────────────────────────\n\
\n\
Define variables:\n\
  (define! x 42)\n\
  x                 => 42\n\
\n\
Lambda (applicative):\n\
  (define! double (lambda (x) (* x 2)))\n\
  (double 21)       => 42\n\
\n\
Conditionals:\n\
  (if #t 1 2)       => 1\n\
  (if #f 1 2)       => 2\n\
  (cond (#f 1) (#t 2))  => 2\n\
\n\
Lists:\n\
  (list 1 2 3)      => (1 2 3)\n\
  (cons 1 (list 2)) => (1 2)\n\
  (car (list 1 2))  => 1\n\
  (cdr (list 1 2))  => (2)\n\
\n\
Operatives (vau / fexprs):\n\
  ($vau (x) e x)    ; raw operative\n\
  (wrap ($vau (x) #ignore x)) ; applicative\n\
\n\
Let bindings:\n\
  (let ((x 1) (y 2)) (+ x y)) => 3\n\
\n\
Sequencing:\n\
  (begin (define! a 1) (+ a 2)) => 3";

const DOC_ADVANCED: &str = "\
Advanced Features\n\
─────────────────\n\
\n\
String operations:\n\
  (string-length \"hello\")     => 5\n\
  (string-append \"hi\" \" \" \"there\") => \"hi there\"\n\
\n\
Higher-order functions:\n\
  (map (lambda (x) (* x x)) (list 1 2 3))\n\
    => (1 4 9)\n\
  (filter (lambda (x) (>? x 2)) (list 1 2 3 4))\n\
    => (3 4)\n\
  (reduce + 0 (list 1 2 3))\n\
    => 6\n\
\n\
Recursion (tail-call optimized):\n\
  (define! fact\n\
    (lambda (n)\n\
      (if (=? n 0) 1\n\
        (* n (fact (- n 1))))))\n\
  (fact 10)           => 3628800\n\
\n\
Boolean logic:\n\
  (and? #t #f)        => #f\n\
  (or? #t #f)         => #t\n\
  (not? #t)           => #f\n\
\n\
Type checking:\n\
  (number? 42)        => #t\n\
  (string? \"hi\")      => #t\n\
  (pair? (list 1))    => #t\n\
  (null? ())          => #t\n\
  (boolean? #t)       => #t";

const SHOWCASE_INFO: &str = "\
Ratzilla & Grift — Mobile Showcase\n\
──────────────────────────────────\n\
\n\
This website is a fully interactive terminal UI running\n\
natively in your mobile browser — no app install needed.\n\
Everything is rendered via WebAssembly + WebGL2.\n\
\n\
Mobile Interactions\n\
───────────────────\n\
\n\
  • Swipe LEFT / RIGHT to switch between tabs\n\
  • Swipe UP / DOWN to scroll content\n\
  • Tap on tabs, links, and buttons to interact\n\
  • Pinch-to-zoom is disabled for native feel\n\
  • Mouse wheel scrolling works on desktop\n\
\n\
Built With\n\
──────────\n\
\n\
  Ratzilla   Terminal web apps with Rust + WASM\n\
  Ratatui    Terminal UI framework for Rust\n\
  TachyonFX  Shader-like effects for terminal UIs\n\
  Grift      Minimalistic Lisp with vau calculus\n\
\n\
Why Terminal UI in the Browser?\n\
──────────────────────────────\n\
\n\
  Traditional web apps use HTML/CSS/JavaScript to render\n\
  DOM elements. This site takes a different approach:\n\
  the entire UI is a Rust application compiled to WASM,\n\
  rendering a terminal grid to an HTML canvas.\n\
\n\
  Benefits:\n\
  • Consistent rendering across all devices\n\
  • No CSS layout quirks or browser differences\n\
  • Rust type safety and performance\n\
  • Retro terminal aesthetic with modern effects\n\
\n\
Mobile-First Design\n\
───────────────────\n\
\n\
  The layout adapts to screen size:\n\
  • Narrow screens get a vertical stacked layout\n\
  • Wide screens get side-by-side panels\n\
  • Touch gestures replace keyboard shortcuts\n\
  • Scrollable sections work on all screen sizes\n\
\n\
  Every section you see can be scrolled by swiping\n\
  up and down, or by using the ▲ / ▼ buttons at\n\
  the bottom of the screen.\n\
\n\
Try the REPL!\n\
─────────────\n\
\n\
  Switch to the REPL tab and type Lisp expressions.\n\
  The on-screen keyboard works — try:\n\
    (+ 1 2)\n\
    (list 1 2 3)\n\
    (define! x 42)\n\
    (* x x)\n\
\n\
  The REPL runs a real Lisp interpreter (Grift)\n\
  compiled to WebAssembly — not a simulation.";

const BLOG_ENTRIES: &[(&str, &str, &str)] = &[
    (
        "Welcome to gold.silver.copper",
        "2025-01-15",
        "Hi! I'm gold.silver.copper — a software developer passionate\n\
         about programming languages, systems programming, and Rust.\n\
         \n\
         This site serves as my personal blog, project showcase, and\n\
         an interactive demo of Grift, my Lisp interpreter.\n\
         \n\
         Everything you see here is rendered as a terminal UI in your\n\
         browser using Ratzilla + TachyonFX + WebAssembly.",
    ),
    (
        "Building Grift: A Minimalistic Lisp",
        "2025-02-01",
        "Grift implements Kernel-style vau calculus with first-class\n\
         operatives that subsume both functions and macros.\n\
         \n\
         Key design goals:\n\
         - Zero unsafe code (#![forbid(unsafe_code)])\n\
         - No heap allocation (arena-only memory)\n\
         - Runs on bare-metal embedded systems\n\
         - Compiles to WebAssembly\n\
         \n\
         All values live in a fixed-size arena with const-generic\n\
         capacity and mark-and-sweep garbage collection.",
    ),
    (
        "Vau Calculus Explained",
        "2025-03-10",
        "Unlike traditional Lisps, Grift uses vau calculus where\n\
         operatives receive their arguments unevaluated along with\n\
         the caller's environment. This makes operatives strictly\n\
         more powerful than macros — they can choose whether and\n\
         when to evaluate each argument.\n\
         \n\
         ($vau (x) env-param body) creates an operative that\n\
         captures the formal parameter tree, environment parameter,\n\
         and body expression as a closure.",
    ),
    (
        "Terminal UIs in the Browser",
        "2025-04-20",
        "This website is built entirely with Ratzilla, which brings\n\
         Ratatui's terminal UI framework to the browser via WASM.\n\
         \n\
         TachyonFX adds shader-like visual effects — the background\n\
         animation, page transitions, and link click effects are all\n\
         powered by tachyonfx running in WebAssembly.\n\
         \n\
         No JavaScript framework. No DOM manipulation. Just Rust\n\
         rendering a terminal buffer to a canvas element.",
    ),
];

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Home,
    Repl,
    Docs,
    Blog,
    Links,
    Showcase,
}

impl Page {
    const ALL: [Page; 6] = [Page::Home, Page::Repl, Page::Docs, Page::Blog, Page::Links, Page::Showcase];

    fn title(self) -> &'static str {
        match self {
            Page::Home => "Home",
            Page::Repl => "REPL",
            Page::Docs => "Docs",
            Page::Blog => "Blog",
            Page::Links => "Links",
            Page::Showcase => "Mobile",
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|&p| p == self).unwrap_or(0)
    }
}

#[derive(Clone, Copy)]
enum ScrollTarget {
    Home,
    Links,
    Showcase,
    Docs,
}

struct App {
    page: Page,
    // REPL state
    repl_input: String,
    repl_cursor: usize,
    repl_history: Vec<(String, String)>,
    repl_scroll: usize,
    lisp: Box<Lisp<2000>>,
    // Docs state
    doc_scroll: usize,
    // Blog state
    blog_index: usize,
    blog_viewing_post: bool,
    // Scroll state for scrollable pages
    home_scroll: usize,
    links_scroll: usize,
    showcase_scroll: usize,
    // Scroll arrow button areas
    scroll_up_area: Rect,
    scroll_down_area: Rect,
    // TachyonFX
    transition_effect: Option<Effect>,
    bg_tick: u64,
    last_frame: web_time::Instant,
    // Grid dimensions (updated each frame from frame.area())
    grid_cols: u16,
    grid_rows: u16,
    // Mouse hover position in grid coordinates
    hover_col: u16,
    hover_row: u16,
    // Mouse trail state
    trail: Vec<(u16, u16, u8)>, // (col, row, intensity)
    mouse_moving: bool,
    mouse_idle_ticks: u16,
    // Cursor blink
    cursor_blink_tick: u64,
    // Clickable area tracking
    tab_area: Rect,
    tab_rects: Vec<Rect>,
    link_areas: Vec<Rect>,
    blog_item_areas: Vec<Rect>,
    blog_back_area: Rect,
    // Zone detection areas
    content_area: Rect,
    blog_list_area: Rect,
    blog_content_area: Rect,
    // Blog scroll
    blog_scroll: usize,
    // Tab horizontal scroll
    tab_h_scroll: usize,
    // Button effects
    btn_effects: Vec<(Rect, Effect)>,
    // Tab glow effect
    tab_glow_effect: Option<Effect>,
    // Tab hover effects
    tab_hover_effects: Vec<(usize, Effect)>,
    last_hovered_tab: Option<usize>,
    // Banner glow effect
    banner_glow_effect: Option<Effect>,
    // Banner area tracking for glow effect
    banner_area: Rect,
    // Link hover effects
    link_hover_effects: Vec<(usize, Effect)>,
    last_hovered_link: Option<usize>,
}

impl App {
    fn new() -> Self {
        let lisp: Box<Lisp<2000>> = Box::new(Lisp::new());
        Self {
            page: Page::Home,
            repl_input: String::new(),
            repl_cursor: 0,
            repl_history: Vec::new(),
            repl_scroll: 0,
            lisp,
            doc_scroll: 0,
            blog_index: 0,
            blog_viewing_post: false,
            blog_scroll: 0,
            home_scroll: 0,
            links_scroll: 0,
            showcase_scroll: 0,
            scroll_up_area: Rect::default(),
            scroll_down_area: Rect::default(),
            transition_effect: None,
            bg_tick: 0,
            last_frame: web_time::Instant::now(),
            grid_cols: 0,
            grid_rows: 0,
            hover_col: 0,
            hover_row: 0,
            trail: Vec::new(),
            mouse_moving: false,
            mouse_idle_ticks: 0,
            cursor_blink_tick: 0,
            tab_area: Rect::default(),
            tab_rects: Vec::new(),
            link_areas: Vec::new(),
            blog_item_areas: Vec::new(),
            blog_back_area: Rect::default(),
            content_area: Rect::default(),
            blog_list_area: Rect::default(),
            blog_content_area: Rect::default(),
            tab_h_scroll: 0,
            btn_effects: Vec::new(),
            tab_glow_effect: None,
            tab_hover_effects: Vec::new(),
            last_hovered_tab: None,
            banner_glow_effect: None,
            banner_area: Rect::default(),
            link_hover_effects: Vec::new(),
            last_hovered_link: None,
        }
    }

    fn trigger_transition(&mut self) {
        let dark = Color::Rgb(8, 9, 14);
        let effect = match self.page {
            Page::Home => fx::fade_from(
                dark,
                dark,
                EffectTimer::from_ms(400, Interpolation::CubicOut),
            ),
            Page::Repl => fx::sweep_in(
                Motion::LeftToRight,
                10,
                3,
                dark,
                EffectTimer::from_ms(500, Interpolation::QuadOut),
            ),
            Page::Docs => fx::slide_in(
                Motion::RightToLeft,
                8,
                3,
                dark,
                EffectTimer::from_ms(500, Interpolation::CubicOut),
            ),
            Page::Blog => fx::coalesce(EffectTimer::from_ms(400, Interpolation::SineOut)),
            Page::Links => fx::sweep_in(
                Motion::UpToDown,
                8,
                2,
                dark,
                EffectTimer::from_ms(500, Interpolation::QuadOut),
            ),
            Page::Showcase => fx::fade_from(
                dark,
                dark,
                EffectTimer::from_ms(500, Interpolation::CubicOut),
            ),
        };
        self.transition_effect = Some(effect);
    }

    fn switch_page(&mut self, page: Page) {
        if self.page != page {
            self.page = page;
            self.tab_glow_effect = None;
            self.blog_scroll = 0;
            self.blog_viewing_post = false;
            self.trigger_transition();
            // Focus/blur hidden input for REPL virtual keyboard
            if page == Page::Repl {
                let _ = web_sys::js_sys::eval("window._replTabActive=true;window._focusReplInput&&window._focusReplInput()");
            } else {
                let _ = web_sys::js_sys::eval("window._replTabActive=false;window._blurReplInput&&window._blurReplInput()");
            }
        }
    }

    fn trigger_btn_effect(&mut self, area: Rect) {
        let effect = fx::fade_from(
            Color::Rgb(140, 145, 160),
            Color::Rgb(30, 35, 50),
            EffectTimer::from_ms(400, Interpolation::CubicOut),
        );
        self.btn_effects.push((area, effect));
    }

    fn trigger_link_effect(&mut self, area: Rect) {
        let dark = Color::Rgb(8, 9, 14);
        let sweep = fx::sweep_in(
            Motion::LeftToRight,
            6,
            2,
            dark,
            EffectTimer::from_ms(400, Interpolation::QuadOut),
        );
        self.btn_effects.push((area, sweep));
        let shift = fx::hsl_shift_fg(
            [25.0, 12.0, 18.0],
            (500, Interpolation::SineOut),
        );
        self.btn_effects.push((area, shift));
    }

    fn is_hovered(&self, area: Rect) -> bool {
        self.hover_col >= area.x
            && self.hover_col < area.right()
            && self.hover_row >= area.y
            && self.hover_row < area.bottom()
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        // If hover is in tab area, horizontal keys scroll tabs
        let in_tab_area = self.tab_area.width > 0
            && self.hover_row >= self.tab_area.y
            && self.hover_row < self.tab_area.bottom();

        if in_tab_area {
            match key.code {
                KeyCode::Left => {
                    self.tab_h_scroll = self.tab_h_scroll.saturating_sub(2);
                    return;
                }
                KeyCode::Right => {
                    self.tab_h_scroll += 2;
                    return;
                }
                _ => {}
            }
        }

        match self.page {
            Page::Repl => self.handle_repl_event(key),
            Page::Docs => self.handle_scroll_event(key, ScrollTarget::Docs),
            Page::Blog => self.handle_blog_event(key),
            Page::Home => self.handle_scroll_event(key, ScrollTarget::Home),
            Page::Links => self.handle_scroll_event(key, ScrollTarget::Links),
            Page::Showcase => self.handle_scroll_event(key, ScrollTarget::Showcase),
        }
    }

    fn handle_mouse_event(&mut self, event: MouseEvent) {
        // The ratzilla MouseEvent already provides terminal grid coordinates
        // (col, row) via beamterm's TerminalMouseHandler — no manual conversion needed.
        let col = event.col;
        let row = event.row;

        // Update hover position on any mouse event
        let prev_col = self.hover_col;
        let prev_row = self.hover_row;
        self.hover_col = col;
        self.hover_row = row;

        // Track mouse movement for cursor trail
        if col != prev_col || row != prev_row {
            self.mouse_moving = true;
            self.mouse_idle_ticks = 0;
            // Add trail point
            self.trail.push((col, row, TRAIL_INITIAL_INTENSITY));
            // Keep trail limited
            if self.trail.len() > MAX_TRAIL_LENGTH {
                self.trail.remove(0);
            }
        }

        if event.kind == MouseEventKind::ButtonDown(MouseButton::Left) {

            // Check tab clicks using individual tab areas
            if row >= self.tab_area.y && row < self.tab_area.bottom() {
                for (i, tab_rect) in self.tab_rects.iter().enumerate() {
                    if col >= tab_rect.x
                        && col < tab_rect.right()
                        && row >= tab_rect.y
                        && row < tab_rect.bottom()
                    {
                        if i < Page::ALL.len() {
                            self.trigger_btn_effect(*tab_rect);
                            self.switch_page(Page::ALL[i]);
                            return;
                        }
                    }
                }
            }

            // Check link clicks on Links page
            if self.page == Page::Links {
                for (i, area) in self.link_areas.iter().enumerate() {
                    if col >= area.x
                        && col < area.right()
                        && row >= area.y
                        && row < area.bottom()
                    {
                        if let Some((_, url)) = LINKS.get(i) {
                            self.trigger_link_effect(*area);
                            self.trigger_transition();
                            open_url(url);
                            return;
                        }
                    }
                }
            }

            // Check blog item clicks
            if self.page == Page::Blog {
                // Check back button click (narrow mode)
                if self.blog_back_area.width > 0
                    && col >= self.blog_back_area.x
                    && col < self.blog_back_area.right()
                    && row >= self.blog_back_area.y
                    && row < self.blog_back_area.bottom()
                {
                    self.blog_viewing_post = false;
                    self.blog_scroll = 0;
                    self.trigger_btn_effect(self.blog_back_area);
                    self.trigger_transition();
                    return;
                }

                for (i, area) in self.blog_item_areas.iter().enumerate() {
                    if col >= area.x
                        && col < area.right()
                        && row >= area.y
                        && row < area.bottom()
                        && i < BLOG_ENTRIES.len()
                    {
                        if self.blog_index != i || !self.blog_viewing_post {
                            self.blog_index = i;
                            self.blog_viewing_post = true;
                            self.blog_scroll = 0;
                            self.trigger_btn_effect(*area);
                            self.trigger_transition();
                        }
                        return;
                    }
                }
            }

            // Check scroll arrow button clicks
            if self.scroll_up_area.width > 0
                && col >= self.scroll_up_area.x
                && col < self.scroll_up_area.right()
                && row >= self.scroll_up_area.y
                && row < self.scroll_up_area.bottom()
            {
                self.trigger_btn_effect(self.scroll_up_area);
                self.handle_key_event(KeyEvent { code: KeyCode::Up, shift: false, ctrl: false, alt: false });
                return;
            }
            if self.scroll_down_area.width > 0
                && col >= self.scroll_down_area.x
                && col < self.scroll_down_area.right()
                && row >= self.scroll_down_area.y
                && row < self.scroll_down_area.bottom()
            {
                self.trigger_btn_effect(self.scroll_down_area);
                self.handle_key_event(KeyEvent { code: KeyCode::Down, shift: false, ctrl: false, alt: false });
                return;
            }
        }
    }

    fn handle_repl_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if !self.repl_input.is_empty() {
                    let input = self.repl_input.clone();
                    let result = match self.lisp.eval_to_index(&input) {
                        Ok(idx) => {
                            let mut buf = String::new();
                            match self.lisp.write_value(idx, &mut buf) {
                                Ok(()) => buf,
                                Err(_) => "<format error>".to_string(),
                            }
                        }
                        Err(e) => format!("Error: {e:?}"),
                    };
                    self.repl_history.push((input, result));
                    self.repl_input.clear();
                    self.repl_cursor = 0;
                    self.repl_scroll = 0;
                }
            }
            KeyCode::Char(c) => {
                let byte_idx = self.byte_index();
                self.repl_input.insert(byte_idx, c);
                self.repl_cursor += 1;
            }
            KeyCode::Backspace => {
                if self.repl_cursor > 0 {
                    let current = self.repl_cursor;
                    let before: String = self.repl_input.chars().take(current - 1).collect();
                    let after: String = self.repl_input.chars().skip(current).collect();
                    self.repl_input = before + &after;
                    self.repl_cursor -= 1;
                }
            }
            KeyCode::Left => {
                self.repl_cursor = self.repl_cursor.saturating_sub(1);
            }
            KeyCode::Right => {
                let max = self.repl_input.chars().count();
                if self.repl_cursor < max {
                    self.repl_cursor += 1;
                }
            }
            KeyCode::Up => {
                self.repl_scroll = self.repl_scroll.saturating_sub(1);
            }
            KeyCode::Down => {
                self.repl_scroll += 1;
            }
            _ => {}
        }
    }

    fn handle_blog_event(&mut self, key: KeyEvent) {
        if self.blog_viewing_post {
            // Viewing a post: scroll content or go back
            match key.code {
                KeyCode::Up => {
                    self.blog_scroll = self.blog_scroll.saturating_sub(1);
                }
                KeyCode::Down => {
                    self.blog_scroll += 1;
                }
                KeyCode::Left => {
                    self.blog_viewing_post = false;
                    self.blog_scroll = 0;
                    self.trigger_transition();
                }
                _ => {}
            }
        } else {
            // Viewing the list: navigate or select
            match key.code {
                KeyCode::Up => {
                    if self.blog_index > 0 {
                        self.blog_index -= 1;
                        self.trigger_transition();
                    }
                }
                KeyCode::Down => {
                    if self.blog_index < BLOG_ENTRIES.len() - 1 {
                        self.blog_index += 1;
                        self.trigger_transition();
                    }
                }
                KeyCode::Right => {
                    self.blog_viewing_post = true;
                    self.blog_scroll = 0;
                    self.trigger_transition();
                }
                _ => {}
            }
        }
    }

    fn handle_scroll_event(&mut self, key: KeyEvent, target: ScrollTarget) {
        let scroll = match target {
            ScrollTarget::Home => &mut self.home_scroll,
            ScrollTarget::Links => &mut self.links_scroll,
            ScrollTarget::Showcase => &mut self.showcase_scroll,
            ScrollTarget::Docs => &mut self.doc_scroll,
        };
        let step = if self.grid_cols < NARROW_WIDTH_THRESHOLD { 2 } else { 1 };
        match key.code {
            KeyCode::Up => {
                *scroll = scroll.saturating_sub(step);
            }
            KeyCode::Down => {
                *scroll += step;
            }
            _ => {}
        }
    }

    fn byte_index(&self) -> usize {
        self.repl_input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.repl_cursor)
            .unwrap_or(self.repl_input.len())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let now = web_time::Instant::now();
        let elapsed_std = now - self.last_frame;
        self.last_frame = now;
        let elapsed = Duration::from_millis(elapsed_std.as_millis() as u32);

        self.bg_tick = self.bg_tick.wrapping_add(1);
        self.cursor_blink_tick = self.cursor_blink_tick.wrapping_add(1);

        // Decay mouse trail
        self.mouse_idle_ticks = self.mouse_idle_ticks.saturating_add(1);
        if self.mouse_idle_ticks > 3 {
            self.mouse_moving = false;
        }
        // Fade trail points
        self.trail.retain_mut(|(_, _, intensity)| {
            *intensity = intensity.saturating_sub(TRAIL_FADE_RATE);
            *intensity > 0
        });

        // Render background animation across entire area
        self.render_background(frame);

        let full_area = frame.area();

        // Store grid dimensions for mouse coordinate conversion
        self.grid_cols = full_area.width;
        self.grid_rows = full_area.height;

        // Center the main content with margins to show animated background border
        // Use smaller margins on narrow screens (phones) for better usability
        let h_margin = if full_area.width <= VERY_NARROW_WIDTH_THRESHOLD {
            0
        } else if full_area.width < NARROW_MARGIN_THRESHOLD {
            1
        } else {
            (full_area.width / 10).max(2)
        };
        let v_margin = if full_area.height < SHORT_MARGIN_THRESHOLD { 0 } else { (full_area.height / 16).max(1) };

        let [_, center_v, _] = Layout::vertical([
            Constraint::Length(v_margin),
            Constraint::Min(10),
            Constraint::Length(v_margin),
        ])
        .areas(full_area);

        let [_, main_area, _] = Layout::horizontal([
            Constraint::Length(h_margin),
            Constraint::Min(20),
            Constraint::Length(h_margin),
        ])
        .areas(center_v);

        let [tab_area, content_area] =
            Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).areas(main_area);

        self.content_area = content_area;

        self.render_tabs(frame, tab_area);

        // Render fire glow effect on the selected tab
        if let Some(selected_tab_rect) = self.tab_rects.get(self.page.index()).copied() {
            if self.tab_glow_effect.is_none() {
                // Subtle warm hsl shift that ping-pongs for a fire-like glow
                let fg_shift = [-330.0, 15.0, 10.0];
                let timer = (1200, Interpolation::SineIn);
                let glow = fx::hsl_shift_fg(fg_shift, timer)
                    .with_filter(CellFilter::Text);
                self.tab_glow_effect = Some(fx::repeating(fx::ping_pong(glow)));
            }
            if let Some(ref mut effect) = self.tab_glow_effect {
                frame.render_effect(effect, selected_tab_rect, elapsed);
            }
        }

        // Tab hover translate effect — triggers when a new tab is hovered
        let current_hovered_tab = self.tab_rects.iter().enumerate()
            .find(|(_, r)| self.is_hovered(**r))
            .map(|(i, _)| i);
        if current_hovered_tab != self.last_hovered_tab {
            if let Some(idx) = current_hovered_tab {
                if self.tab_rects.get(idx).is_some() {
                    let inner_effect = fx::fade_from(
                        Color::Rgb(60, 65, 80),
                        Color::Rgb(8, 9, 14),
                        (300, Interpolation::QuadOut),
                    );
                    let hover_fx = fx::translate(
                        inner_effect,
                        Offset { x: 0, y: -1 },
                        (300, Interpolation::QuadOut),
                    );
                    self.tab_hover_effects.push((idx, hover_fx));
                }
            }
            self.last_hovered_tab = current_hovered_tab;
        }
        // Process tab hover effects
        self.tab_hover_effects.retain_mut(|(idx, effect)| {
            if effect.running() {
                if let Some(tab_rect) = self.tab_rects.get(*idx).copied() {
                    frame.render_effect(effect, tab_rect, elapsed);
                }
                true
            } else {
                false
            }
        });

        match self.page {
            Page::Home => self.render_home(frame, content_area),
            Page::Repl => self.render_repl(frame, content_area),
            Page::Docs => self.render_docs(frame, content_area),
            Page::Blog => self.render_blog(frame, content_area),
            Page::Links => self.render_links(frame, content_area),
            Page::Showcase => self.render_showcase(frame, content_area),
        }

        // Process transition effects
        if let Some(ref mut effect) = self.transition_effect {
            if effect.running() {
                frame.render_effect(effect, content_area, elapsed);
            }
        }
        if self
            .transition_effect
            .as_ref()
            .is_some_and(|e| !e.running())
        {
            self.transition_effect = None;
        }

        // Process button effects
        self.btn_effects.retain_mut(|(area, effect)| {
            if effect.running() {
                frame.render_effect(effect, *area, elapsed);
                true
            } else {
                false
            }
        });

        // Link hover effects — triggers when a new link is hovered
        if self.page == Page::Links {
            let current_hovered_link = self.link_areas.iter().enumerate()
                .find(|(_, r)| self.is_hovered(**r))
                .map(|(i, _)| i);
            if current_hovered_link != self.last_hovered_link {
                if let Some(idx) = current_hovered_link {
                    if let Some(link_rect) = self.link_areas.get(idx).copied() {
                        let hover_fx = fx::fade_from(
                            Color::Rgb(80, 90, 110),
                            Color::Rgb(8, 9, 14),
                            (400, Interpolation::QuadOut),
                        );
                        self.link_hover_effects.push((idx, hover_fx));
                        // Also trigger a subtle hsl shift
                        let shift_fx = fx::hsl_shift_fg(
                            [15.0, 8.0, 10.0],
                            (500, Interpolation::SineOut),
                        );
                        self.btn_effects.push((link_rect, shift_fx));
                    }
                }
                self.last_hovered_link = current_hovered_link;
            }
            // Process link hover effects
            self.link_hover_effects.retain_mut(|(idx, effect)| {
                if effect.running() {
                    if let Some(link_rect) = self.link_areas.get(*idx).copied() {
                        frame.render_effect(effect, link_rect, elapsed);
                    }
                    true
                } else {
                    false
                }
            });
        }

        // Subtle continuous glow on banner when on Home page
        if self.page == Page::Home && self.banner_area.width > 0 {
            if self.banner_glow_effect.is_none() {
                let fg_shift = [-330.0, 10.0, 8.0];
                let timer = (2000, Interpolation::SineIn);
                let glow = fx::hsl_shift_fg(fg_shift, timer)
                    .with_filter(CellFilter::Text);
                self.banner_glow_effect = Some(fx::repeating(fx::ping_pong(glow)));
            }
            if let Some(ref mut effect) = self.banner_glow_effect {
                // Apply only to the banner area, not the entire content area
                frame.render_effect(effect, self.banner_area, elapsed);
            }
        }

        // Render cursor trail
        if self.mouse_moving || !self.trail.is_empty() {
            let buf = frame.buffer_mut();
            for &(tx, ty, intensity) in &self.trail {
                let pos = Position::new(tx, ty);
                if let Some(cell) = buf.cell_mut(pos) {
                    let boost = (intensity as u16 / TRAIL_FADE_RATE as u16) as u8;
                    let (r, g, b) = match cell.bg {
                        Color::Rgb(r, g, b) => (r, g, b),
                        _ => (8, 9, 14),
                    };
                    cell.set_bg(Color::Rgb(
                        r.saturating_add(boost / 2),
                        g.saturating_add(boost / 2),
                        b.saturating_add(boost),
                    ));
                }
            }
        }
    }

    fn render_background(&self, frame: &mut Frame) {
        let area = frame.area();
        let buf = frame.buffer_mut();
        let tick = self.bg_tick;

        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                let pos = Position::new(x, y);
                if let Some(cell) = buf.cell_mut(pos) {
                    // Procedural calm wave — cool silver/steel tones
                    let fx = x as f64 * 0.12;
                    let fy = y as f64 * 0.25;
                    let ft = tick as f64 * 0.015;

                    let wave1 = ((fx + ft).sin() * 0.5 + 0.5) * 0.35;
                    let wave2 = ((fy * 0.6 + ft * 1.1).cos() * 0.5 + 0.5) * 0.35;
                    let wave3 = ((fx * 0.4 + fy * 0.4 + ft * 0.5).sin() * 0.5 + 0.5) * 0.3;

                    let intensity = wave1 + wave2 + wave3;

                    let base = 6.0;
                    let r = (base + intensity * 16.0) as u8;
                    let g = (base + intensity * 18.0) as u8;
                    let b = (base + 2.0 + intensity * 24.0) as u8;

                    cell.set_bg(Color::Rgb(r, g, b));
                    cell.set_fg(Color::Rgb(
                        (r as u16 + 10).min(255) as u8,
                        (g as u16 + 12).min(255) as u8,
                        (b as u16 + 14).min(255) as u8,
                    ));
                }
            }
        }
    }

    fn render_tabs(&mut self, frame: &mut Frame, area: Rect) {
        self.tab_area = area;

        let is_narrow = area.width < NARROW_WIDTH_THRESHOLD;

        // Compute individual tab click areas from the Tabs widget layout.
        // Use narrower dividers and less padding on mobile for compact layout.
        let divider_width: u16 = if is_narrow { 1 } else { 3 };
        let tab_padding: u16 = if is_narrow { 0 } else { 2 };
        let inner_x = area.x + 1;
        let tab_row = area.y + 1;
        self.tab_rects.clear();
        let mut pos = inner_x;
        for p in &Page::ALL {
            let title_len = p.title().len() as u16;
            let total = title_len + tab_padding;
            self.tab_rects.push(Rect::new(pos, tab_row, total, 1));
            pos += total + divider_width;
        }

        // Clamp horizontal scroll
        let total_tab_width = self.tab_rects.last()
            .map(|r| r.right().saturating_sub(area.x))
            .unwrap_or(0);
        let visible_width = area.width.saturating_sub(2);
        let max_h_scroll = total_tab_width.saturating_sub(visible_width) as usize;
        self.tab_h_scroll = self.tab_h_scroll.min(max_h_scroll);

        let divider_str = if is_narrow { "│" } else { " │ " };

        let mut spans: Vec<Span> = Vec::new();
        for (i, p) in Page::ALL.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(divider_str, Style::default().fg(Color::Rgb(100, 105, 115))));
            }
            let hovered = self.tab_rects.get(i).is_some_and(|r| self.is_hovered(*r));
            let is_selected = self.page.index() == i;
            let fg = if is_selected {
                Color::Rgb(230, 232, 240)
            } else if hovered {
                Color::Rgb(255, 255, 255)
            } else {
                Color::Rgb(140, 145, 155)
            };
            let style = if is_selected {
                Style::default().fg(fg).bold().add_modifier(Modifier::REVERSED)
            } else if hovered {
                Style::default().fg(fg).bold().add_modifier(Modifier::UNDERLINED)
            } else {
                Style::default().fg(fg)
            };
            if !is_narrow {
                spans.push(Span::styled(" ", Style::default()));
            }
            spans.push(Span::styled(p.title(), style));
            if !is_narrow {
                spans.push(Span::styled(" ", Style::default()));
            }
        }

        let tab_line = Line::from(spans);
        let tab_paragraph = Paragraph::new(tab_line)
            .scroll((0, self.tab_h_scroll as u16))
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Color::Rgb(55, 60, 70))
                    .title(" GRIFT.RS ")
                    .title_style(Style::default().fg(Color::Rgb(207, 181, 59)).bold()),
            );

        frame.render_widget(tab_paragraph, area);
    }

    fn render_home(&mut self, frame: &mut Frame, area: Rect) {
        // Combine all sections into one scrollable text
        let mut lines: Vec<Line> = Vec::new();

        // Banner (only on wider screens)
        if area.width >= NARROW_WIDTH_THRESHOLD {
            for l in BANNER.lines() {
                lines.push(Line::styled(l, Style::default().fg(Color::Rgb(200, 200, 210)).bold()));
            }
            lines.push(Line::from(""));

            // Description
            lines.push(Line::styled("┌─ Grift ──────────────┐", Style::default().fg(Color::Rgb(184, 115, 51)).bold()));
            for l in DESCRIPTION.lines() {
                lines.push(Line::styled(l, Style::default().fg(Color::Rgb(170, 175, 185))));
            }
            lines.push(Line::from(""));
        }

        // About header
        lines.push(Line::styled("┌─ About ──────────────┐", Style::default().fg(Color::Rgb(200, 200, 210)).bold()));
        for l in ABOUT.lines() {
            lines.push(Line::styled(l, Style::default().fg(Color::Rgb(140, 145, 155))));
        }
        lines.push(Line::from(""));

        // Lisp header
        lines.push(Line::styled("┌─ Lisp ───────────────┐", Style::default().fg(Color::Rgb(207, 181, 59)).bold()));
        for l in LISP_INFO.lines() {
            lines.push(Line::styled(l, Style::default().fg(Color::Rgb(170, 175, 185))));
        }
        lines.push(Line::from(""));

        // Vau calculus header
        lines.push(Line::styled("┌─ Vau Calculus ───────┐", Style::default().fg(Color::Rgb(184, 115, 51)).bold()));
        for l in VAU_INFO.lines() {
            lines.push(Line::styled(l, Style::default().fg(Color::Rgb(170, 175, 185))));
        }
        lines.push(Line::from(""));

        // Rust header
        lines.push(Line::styled("┌─ Rust ───────────────┐", Style::default().fg(Color::Rgb(222, 165, 132)).bold()));
        for l in RUST_INFO.lines() {
            lines.push(Line::styled(l, Style::default().fg(Color::Rgb(170, 175, 185))));
        }
        lines.push(Line::from(""));

        // gold.silver.copper header
        lines.push(Line::styled("┌─ gold.silver.copper ─┐", Style::default().fg(Color::Rgb(207, 181, 59)).bold()));
        for l in GSC_INFO.lines() {
            lines.push(Line::styled(l, Style::default().fg(Color::Rgb(170, 175, 185))));
        }

        let mut scroll = self.home_scroll;
        let scroll_area = self.render_scrollable_content(
            frame, area, lines, &mut scroll,
            None,
            None,
            "swipe ↕ ↔",
        );
        self.home_scroll = scroll;

        // Track banner area for glow effect
        self.banner_area = scroll_area;
    }

    fn render_repl(&self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(55, 60, 70))
            .title(" Grift REPL ".bold().fg(Color::Rgb(184, 115, 51)))
            .title_bottom(
                Line::from("│ type + Enter │")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::Rgb(55, 60, 70))),
            );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let [input_area, history_area] =
            Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).areas(inner);

        // Input (at the top)
        let input_display = format!("Λ> {}", self.repl_input);
        let input = Paragraph::new(input_display.as_str())
            .style(Style::default().fg(Color::Rgb(200, 200, 210)))
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Color::Rgb(184, 115, 51))
                    .title(" Input ".bold().fg(Color::Rgb(184, 115, 51))),
            );
        frame.render_widget(input, input_area);

        // Blinking block cursor
        self.render_blinking_cursor(frame, input_area.x + 1 + 3 + self.repl_cursor as u16, input_area.y + 1, input_area.right() - 1);

        // History (newest first)
        let mut history_lines: Vec<Line> = Vec::new();
        for (input, output) in self.repl_history.iter().rev() {
            history_lines.push(Line::from(vec![
                Span::styled(
                    "Λ> ",
                    Style::default().fg(Color::Rgb(184, 115, 51)).bold(),
                ),
                Span::styled(input.as_str(), Style::default().fg(Color::Rgb(200, 200, 210))),
            ]));
            history_lines.push(Line::from(vec![Span::styled(
                format!("  => {output}"),
                Style::default().fg(Color::Rgb(160, 165, 175)),
            )]));
        }

        if history_lines.is_empty() {
            history_lines.push(Line::from(
                "  Welcome to the Grift REPL! Type expressions and press Enter."
                    .fg(Color::Rgb(100, 105, 115)),
            ));
            history_lines.push(Line::from(
                "  Try: (+ 1 2), (list 1 2 3), (define! x 42)"
                    .fg(Color::Rgb(100, 105, 115)),
            ));
        }

        let visible_height = history_area.height as usize;
        let total_lines = history_lines.len();
        let max_scroll = total_lines.saturating_sub(visible_height);
        let scroll = self.repl_scroll.min(max_scroll);

        let history = Paragraph::new(Text::from(history_lines))
            .wrap(Wrap { trim: false })
            .scroll((scroll as u16, 0))
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Color::Rgb(40, 44, 52))
                    .title(" Output ".fg(Color::Rgb(160, 165, 175))),
            );
        frame.render_widget(history, history_area);
    }

    fn render_blinking_cursor(&self, frame: &mut Frame, cursor_x: u16, cursor_y: u16, max_x: u16) {
        if cursor_x < max_x {
            // Slow blink: visible ~60% of the time
            let blink_on = (self.cursor_blink_tick / CURSOR_BLINK_RATE).is_multiple_of(2);
            if blink_on {
                let buf = frame.buffer_mut();
                let pos = Position::new(cursor_x, cursor_y);
                if let Some(cell) = buf.cell_mut(pos) {
                    // Reverse the cell to show cursor position
                    let fg = cell.fg;
                    let bg = cell.bg;
                    cell.set_fg(bg);
                    cell.set_bg(fg);
                    // Ensure visible even on empty cells
                    if cell.symbol() == " " || cell.symbol().is_empty() {
                        cell.set_bg(Color::Rgb(200, 200, 210));
                        cell.set_fg(Color::Rgb(8, 9, 14));
                    }
                }
            }
        }
    }

    fn render_docs(&mut self, frame: &mut Frame, area: Rect) {
        // Combine all doc sections into one scrollable text
        let all_docs = [DOC_BASICS, DOC_FORMS, DOC_ADVANCED];
        let mut lines: Vec<Line> = Vec::new();
        // Account for outer block borders (2) + inner block borders (2) + side padding (4)
        let separator_width = area.width.saturating_sub(8) as usize;

        for (idx, doc) in all_docs.iter().enumerate() {
            if idx > 0 {
                lines.push(Line::from(""));
                lines.push(Line::styled(
                    "━".repeat(separator_width),
                    Style::default().fg(Color::Rgb(55, 60, 70)),
                ));
                lines.push(Line::from(""));
            }

            for line in doc.lines() {
                if line.starts_with("  (") || line.starts_with("    (") {
                    if let Some((code, comment)) = line.split_once(';') {
                        if let Some((expr, result)) = code.split_once("=>") {
                            lines.push(Line::from(vec![
                                Span::styled(expr, Style::default().fg(Color::Rgb(200, 200, 210))),
                                Span::styled("=>", Style::default().fg(Color::Rgb(80, 85, 95))),
                                Span::styled(result, Style::default().fg(Color::Rgb(160, 165, 175))),
                                Span::styled(format!(";{comment}"), Style::default().fg(Color::Rgb(75, 80, 90))),
                            ]));
                        } else {
                            lines.push(Line::from(vec![
                                Span::styled(code, Style::default().fg(Color::Rgb(200, 200, 210))),
                                Span::styled(format!(";{comment}"), Style::default().fg(Color::Rgb(75, 80, 90))),
                            ]));
                        }
                    } else if let Some((expr, result)) = line.split_once("=>") {
                        lines.push(Line::from(vec![
                            Span::styled(expr, Style::default().fg(Color::Rgb(200, 200, 210))),
                            Span::styled("=>", Style::default().fg(Color::Rgb(80, 85, 95))),
                            Span::styled(result, Style::default().fg(Color::Rgb(160, 165, 175))),
                        ]));
                    } else {
                        lines.push(Line::styled(line, Style::default().fg(Color::Rgb(200, 200, 210))));
                    }
                } else if line.starts_with("  ") && !line.trim().is_empty() {
                    if let Some((code, comment)) = line.split_once(';') {
                        lines.push(Line::from(vec![
                            Span::styled(code, Style::default().fg(Color::Rgb(200, 200, 210))),
                            Span::styled(format!(";{comment}"), Style::default().fg(Color::Rgb(75, 80, 90))),
                        ]));
                    } else if let Some((expr, result)) = line.split_once("=>") {
                        lines.push(Line::from(vec![
                            Span::styled(expr, Style::default().fg(Color::Rgb(200, 200, 210))),
                            Span::styled("=>", Style::default().fg(Color::Rgb(80, 85, 95))),
                            Span::styled(result, Style::default().fg(Color::Rgb(160, 165, 175))),
                        ]));
                    } else {
                        lines.push(Line::styled(line, Style::default().fg(Color::Rgb(200, 200, 210))));
                    }
                } else if line.contains('─') {
                    lines.push(Line::styled(line, Style::default().fg(Color::Rgb(140, 145, 155))));
                } else if !line.trim().is_empty() {
                    lines.push(Line::styled(line, Style::default().fg(Color::Rgb(220, 225, 235)).bold()));
                } else {
                    lines.push(Line::from(""));
                }
            }
        }

        let mut scroll = self.doc_scroll;
        self.render_scrollable_content(
            frame, area, lines, &mut scroll,
            Some(" Documentation ".bold().fg(Color::Rgb(200, 200, 210)).into()),
            Some(" Grift Language Reference ".bold().fg(Color::Rgb(184, 115, 51)).into()),
            "swipe ↕",
        );
        self.doc_scroll = scroll;
    }

    fn render_blog(&mut self, frame: &mut Frame, area: Rect) {
        self.blog_back_area = Rect::default();

        if self.blog_viewing_post {
            // Show post content with a back button
            let block = Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Color::Rgb(55, 60, 70))
                .title(" Blog ".bold().fg(Color::Rgb(200, 200, 210)));

            let inner = block.inner(area);
            frame.render_widget(block, area);

            let [back_bar, scroll_area, nav_bar] =
                Layout::vertical([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)]).areas(inner);

            // Back button
            let back_hovered = self.is_hovered(back_bar);
            let back_style = if back_hovered {
                Style::default().fg(Color::Rgb(255, 255, 255)).bold()
            } else {
                Style::default().fg(Color::Rgb(184, 115, 51))
            };
            frame.render_widget(
                Paragraph::new("◄ Back to posts").style(back_style),
                back_bar,
            );
            self.blog_back_area = back_bar;

            // Blog content — scrollable
            self.blog_list_area = Rect::default();
            self.blog_item_areas.clear();
            if let Some((title, date, content)) = BLOG_ENTRIES.get(self.blog_index) {
                let mut lines = vec![
                    Line::styled(*title, Style::default().fg(Color::Rgb(220, 225, 235)).bold()),
                    Line::styled(*date, Style::default().fg(Color::Rgb(75, 80, 90))),
                    Line::from(""),
                ];
                for line in content.lines() {
                    lines.push(Line::styled(line, Style::default().fg(Color::Rgb(170, 175, 185))));
                }

                let total_lines = lines.len();
                let visible_height = scroll_area.height.saturating_sub(2) as usize;
                let max_scroll = total_lines.saturating_sub(visible_height);
                self.blog_scroll = self.blog_scroll.min(max_scroll);

                let blog = Paragraph::new(Text::from(lines))
                    .wrap(Wrap { trim: false })
                    .scroll((self.blog_scroll as u16, 0))
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .border_style(Color::Rgb(40, 44, 52)),
                    );
                frame.render_widget(blog, scroll_area);
                self.blog_content_area = scroll_area;

                self.render_scroll_arrows(frame, nav_bar, self.blog_scroll, max_scroll, "swipe ↕");
            }
            return;
        }

        // Show scrollable list of blog titles
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(55, 60, 70))
            .title(" Blog ".bold().fg(Color::Rgb(200, 200, 210)))
            .title_bottom(
                Line::from("│ tap a post │")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::Rgb(55, 60, 70))),
            );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let list_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(40, 44, 52))
            .title(" Posts ".fg(Color::Rgb(200, 200, 210)));

        let list_inner = list_block.inner(inner);

        self.blog_item_areas.clear();
        self.blog_list_area = inner;
        self.blog_content_area = Rect::default();

        for i in 0..BLOG_ENTRIES.len() {
            let item_y = list_inner.y + (i as u16 * 2);
            if item_y + 2 <= list_inner.bottom() {
                self.blog_item_areas.push(Rect::new(
                    list_inner.x,
                    item_y,
                    list_inner.width,
                    2,
                ));
            }
        }

        let items: Vec<ListItem> = BLOG_ENTRIES
            .iter()
            .enumerate()
            .map(|(i, (title, date, _))| {
                let hovered = self
                    .blog_item_areas
                    .get(i)
                    .is_some_and(|r| self.is_hovered(*r));
                let style = if i == self.blog_index {
                    Style::default().fg(Color::Rgb(230, 232, 240)).bold()
                } else if hovered {
                    Style::default().fg(Color::Rgb(200, 200, 210))
                } else {
                    Style::default().fg(Color::Rgb(140, 145, 155))
                };
                let marker = if i == self.blog_index { "> " } else if hovered { "~ " } else { "  " };
                ListItem::new(vec![
                    Line::from(format!("{marker}{title}")).style(style),
                    Line::from(format!("  {date}"))
                        .style(Style::default().fg(Color::Rgb(75, 80, 90))),
                ])
            })
            .collect();

        let list = List::new(items).block(list_block);
        frame.render_widget(list, inner);
    }

    fn render_links(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(55, 60, 70))
            .title(" Links ".bold().fg(Color::Rgb(200, 200, 210)));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let [scroll_area, nav_bar] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(inner);

        // Build all links + info as one scrollable text
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::styled("Repositories & Resources", Style::default().fg(Color::Rgb(200, 200, 210)).bold()));
        lines.push(Line::styled("────────────────────────", Style::default().fg(Color::Rgb(140, 145, 155))));

        for (i, (label, _url)) in LINKS.iter().enumerate() {
            let hovered = self.link_areas.get(i).is_some_and(|r| self.is_hovered(*r));
            let style = if hovered {
                Style::default().fg(Color::Rgb(160, 175, 195)).add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(Color::Rgb(160, 175, 195))
            };
            lines.push(Line::styled(format!("  {label}"), style));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            "  gold.silver.copper ".fg(Color::Rgb(207, 181, 59)).bold(),
            "— Software developer".fg(Color::Rgb(140, 145, 155)),
        ]));
        lines.push(Line::from(vec![
            "  Grift ".fg(Color::Rgb(184, 115, 51)).bold(),
            "– Lisp with vau calculus".fg(Color::Rgb(140, 145, 155)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from("  Terminal UI in your browser.".fg(Color::Rgb(100, 105, 115))));
        lines.push(Line::from("  Ratzilla + TachyonFX + WASM.".fg(Color::Rgb(100, 105, 115))));

        let total_lines = lines.len();
        let visible_height = scroll_area.height.saturating_sub(2) as usize;
        let max_scroll = total_lines.saturating_sub(visible_height);
        self.links_scroll = self.links_scroll.min(max_scroll);

        // Track clickable link areas in scroll view
        let links_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(40, 44, 52));
        let links_inner = links_block.inner(scroll_area);

        self.link_areas.clear();
        for i in 0..LINKS.len() {
            let line_idx = i + 2; // offset for header lines
            if line_idx >= self.links_scroll {
                let visible_row = (line_idx - self.links_scroll) as u16;
                if visible_row < links_inner.height {
                    let link_area = Rect::new(links_inner.x, links_inner.y + visible_row, links_inner.width, 1);
                    self.link_areas.push(link_area);
                }
            }
        }

        let content = Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .scroll((self.links_scroll as u16, 0))
            .block(links_block);
        frame.render_widget(content, scroll_area);

        self.render_scroll_arrows(frame, nav_bar, self.links_scroll, max_scroll, "tap to open");
    }

    fn render_scrollable_content(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        lines: Vec<Line<'static>>,
        scroll: &mut usize,
        outer_title: Option<Line<'static>>,
        inner_title: Option<Line<'static>>,
        hint: &str,
    ) -> Rect {
        let mut block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(55, 60, 70));
        if let Some(title) = outer_title {
            block = block.title(title);
        }

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let [scroll_area, nav_bar] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(inner);

        let total_lines = lines.len();
        let visible_height = scroll_area.height.saturating_sub(2) as usize;
        let max_scroll = total_lines.saturating_sub(visible_height);
        *scroll = (*scroll).min(max_scroll);

        let mut content_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(40, 44, 52));
        if let Some(title) = inner_title {
            content_block = content_block.title(title);
        }

        let content = Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .scroll((*scroll as u16, 0))
            .block(content_block);
        frame.render_widget(content, scroll_area);

        self.render_scroll_arrows(frame, nav_bar, *scroll, max_scroll, hint);

        scroll_area
    }

    fn render_scroll_arrows(&mut self, frame: &mut Frame, nav_bar: Rect, scroll: usize, max_scroll: usize, hint: &str) {
        let arrow_width: u16 = 3;
        let [up_area, center_area, down_area] = Layout::horizontal([
            Constraint::Length(arrow_width),
            Constraint::Min(1),
            Constraint::Length(arrow_width),
        ])
        .areas(nav_bar);

        self.scroll_up_area = up_area;
        self.scroll_down_area = down_area;

        let can_up = scroll > 0;
        let can_down = scroll < max_scroll;

        let up_hovered = can_up && self.is_hovered(up_area);
        let down_hovered = can_down && self.is_hovered(down_area);

        let up_style = if up_hovered {
            Style::default().fg(Color::Rgb(255, 255, 255)).bold()
        } else if can_up {
            Style::default().fg(Color::Rgb(140, 145, 155))
        } else {
            Style::default().fg(Color::Rgb(35, 38, 46))
        };
        let down_style = if down_hovered {
            Style::default().fg(Color::Rgb(255, 255, 255)).bold()
        } else if can_down {
            Style::default().fg(Color::Rgb(140, 145, 155))
        } else {
            Style::default().fg(Color::Rgb(35, 38, 46))
        };

        frame.render_widget(
            Paragraph::new(" ▲ ").style(up_style).alignment(Alignment::Center),
            up_area,
        );
        frame.render_widget(
            Paragraph::new(" ▼ ").style(down_style).alignment(Alignment::Center),
            down_area,
        );

        // Center area: hint + scroll position
        let indicator = if max_scroll > 0 {
            format!("{}/{}", scroll + 1, max_scroll + 1)
        } else {
            "─".to_string()
        };
        let center_text = format!("{hint} │ {indicator}");
        frame.render_widget(
            Paragraph::new(center_text)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Rgb(75, 80, 90))),
            center_area,
        );
    }

    fn render_showcase(&mut self, frame: &mut Frame, area: Rect) {
        // Render showcase content with syntax highlighting for headers
        let lines: Vec<Line> = SHOWCASE_INFO
            .lines()
            .map(|line| {
                if line.contains('─') && !line.starts_with(' ') {
                    Line::styled(line, Style::default().fg(Color::Rgb(140, 145, 155)))
                } else if !line.starts_with(' ') && !line.is_empty() {
                    Line::styled(line, Style::default().fg(Color::Rgb(220, 225, 235)).bold())
                } else if line.starts_with("  •") {
                    let parts: Vec<&str> = line.splitn(2, '•').collect();
                    if parts.len() == 2 {
                        Line::from(vec![
                            Span::styled("  •", Style::default().fg(Color::Rgb(207, 181, 59))),
                            Span::styled(parts[1], Style::default().fg(Color::Rgb(170, 175, 185))),
                        ])
                    } else {
                        Line::styled(line, Style::default().fg(Color::Rgb(170, 175, 185)))
                    }
                } else if line.starts_with("  ") && line.contains("   ") {
                    // Two-column items like "  Ratzilla   Terminal web apps..."
                    let trimmed = line.trim_start();
                    if let Some(idx) = trimmed.find("   ") {
                        let name = &trimmed[..idx];
                        let desc = trimmed[idx..].trim_start();
                        Line::from(vec![
                            Span::styled("  ", Style::default()),
                            Span::styled(name, Style::default().fg(Color::Rgb(184, 115, 51)).bold()),
                            Span::styled("  ", Style::default()),
                            Span::styled(desc, Style::default().fg(Color::Rgb(140, 145, 155))),
                        ])
                    } else {
                        Line::styled(line, Style::default().fg(Color::Rgb(170, 175, 185)))
                    }
                } else {
                    Line::styled(line, Style::default().fg(Color::Rgb(170, 175, 185)))
                }
            })
            .collect();

        let mut scroll = self.showcase_scroll;
        self.render_scrollable_content(
            frame, area, lines, &mut scroll,
            Some(" Mobile Showcase ".bold().fg(Color::Rgb(207, 181, 59)).into()),
            Some(" Ratzilla & Grift ".bold().fg(Color::Rgb(184, 115, 51)).into()),
            "swipe ↕",
        );
        self.showcase_scroll = scroll;
    }
}

fn open_url(url: &str) {
    // Open in a new background tab using web_sys directly (avoids JS string interpolation)
    if let Some(window) = web_sys::window() {
        let _ = window.open_with_url_and_target_and_features(url, "_blank", "noopener");
        let _ = window.focus();
    }
}

fn main() -> std::io::Result<()> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let app = Rc::new(RefCell::new(App::new()));

    // Use WebGl2Backend with mouse selection for text select and copy/paste.
    macro_rules! setup_terminal {
        ($terminal:expr, $app:expr) => {{
            $terminal.on_key_event({
                let app = $app.clone();
                move |key_event| { app.borrow_mut().handle_key_event(key_event); }
            }).expect("failed to register key event handler");
            $terminal.on_mouse_event({
                let app = $app.clone();
                move |mouse_event| { app.borrow_mut().handle_mouse_event(mouse_event); }
            }).expect("failed to register mouse event handler");
            $terminal.draw_web({
                let app = $app.clone();
                move |frame| { app.borrow_mut().draw(frame); }
            });
        }};
    }

    let options = WebGl2BackendOptions::new()
        .enable_mouse_selection_with_mode(Default::default());
    let backend = WebGl2Backend::new_with_options(options).expect("failed to create WebGL2 backend");
    let mut terminal = ratzilla::ratatui::Terminal::new(backend)?;
    setup_terminal!(terminal, app);

    Ok(())
}
