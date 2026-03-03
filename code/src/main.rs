use std::cell::RefCell;
use std::rc::Rc;

use grift::Lisp;
use ratzilla::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratzilla::ratatui::layout::{Alignment, Constraint, Layout, Offset, Position, Rect};
use ratzilla::ratatui::style::{Color, Modifier, Style, Stylize};
use ratzilla::ratatui::text::{Line, Span, Text};
use ratzilla::ratatui::widgets::{Block, BorderType, List, ListItem, Paragraph, Tabs, Wrap};
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
}

impl Page {
    const ALL: [Page; 5] = [Page::Home, Page::Repl, Page::Docs, Page::Blog, Page::Links];

    fn title(self) -> &'static str {
        match self {
            Page::Home => "Home",
            Page::Repl => "REPL",
            Page::Docs => "Docs",
            Page::Blog => "Blog",
            Page::Links => "Links",
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|&p| p == self).unwrap_or(0)
    }
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
    doc_page: usize,
    // Blog state
    blog_index: usize,
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
    doc_nav_prev: Rect,
    doc_nav_next: Rect,
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
            doc_page: 0,
            blog_index: 0,
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
            doc_nav_prev: Rect::default(),
            doc_nav_next: Rect::default(),
            btn_effects: Vec::new(),
            tab_glow_effect: None,
            tab_hover_effects: Vec::new(),
            last_hovered_tab: None,
            banner_glow_effect: None,
            banner_area: Rect::default(),
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
        };
        self.transition_effect = Some(effect);
    }

    fn switch_page(&mut self, page: Page) {
        if self.page != page {
            self.page = page;
            self.tab_glow_effect = None; // Reset so it restarts on new tab
            self.trigger_transition();
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
        let effect = fx::hsl_shift_fg(
            [20.0, 10.0, 15.0],
            (300, Interpolation::SineOut),
        );
        self.btn_effects.push((area, effect));
    }

    fn is_hovered(&self, area: Rect) -> bool {
        self.hover_col >= area.x
            && self.hover_col < area.right()
            && self.hover_row >= area.y
            && self.hover_row < area.bottom()
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        match self.page {
            Page::Repl => self.handle_repl_event(key),
            Page::Docs => self.handle_docs_event(key),
            Page::Blog => self.handle_blog_event(key),
            _ => {}
        }
    }

    fn handle_mouse_event(&mut self, event: MouseEvent) {
        // Dynamically convert pixel coordinates to grid coordinates using
        // the actual window dimensions and grid size from the last frame.
        let col;
        let row;
        {
            let window = web_sys::window().expect("no global window in WASM context");
            let win_w = window
                .inner_width()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(800.0)
                .max(1.0);
            let win_h = window
                .inner_height()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(600.0)
                .max(1.0);
            let cols = if self.grid_cols == 0 {
                ratzilla::utils::get_window_size().width
            } else {
                self.grid_cols
            };
            let rows = if self.grid_rows == 0 {
                ratzilla::utils::get_window_size().height
            } else {
                self.grid_rows
            };
            col = ((event.x as f64 / win_w) * cols as f64) as u16;
            row = ((event.y as f64 / win_h) * rows as f64) as u16;
        }

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

        if event.event == MouseEventKind::Pressed && event.button == MouseButton::Left {

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
                for (i, area) in self.blog_item_areas.iter().enumerate() {
                    if col >= area.x
                        && col < area.right()
                        && row >= area.y
                        && row < area.bottom()
                        && i < BLOG_ENTRIES.len()
                        && self.blog_index != i
                    {
                        self.blog_index = i;
                        self.trigger_btn_effect(*area);
                        self.trigger_transition();
                        return;
                    }
                }
            }

            // Check doc navigation buttons
            if self.page == Page::Docs {
                if col >= self.doc_nav_prev.x
                    && col < self.doc_nav_prev.right()
                    && row >= self.doc_nav_prev.y
                    && row < self.doc_nav_prev.bottom()
                    && self.doc_page > 0
                {
                    self.trigger_btn_effect(self.doc_nav_prev);
                    self.doc_page -= 1;
                    self.trigger_transition();
                }
                if col >= self.doc_nav_next.x
                    && col < self.doc_nav_next.right()
                    && row >= self.doc_nav_next.y
                    && row < self.doc_nav_next.bottom()
                    && self.doc_page < 2
                {
                    self.trigger_btn_effect(self.doc_nav_next);
                    self.doc_page += 1;
                    self.trigger_transition();
                }
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
                    let total = self.repl_history.len() * 2;
                    self.repl_scroll = total;
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

    fn handle_docs_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Left => {
                if self.doc_page > 0 {
                    self.doc_page -= 1;
                    self.trigger_transition();
                }
            }
            KeyCode::Right => {
                if self.doc_page < 2 {
                    self.doc_page += 1;
                    self.trigger_transition();
                }
            }
            _ => {}
        }
    }

    fn handle_blog_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Left | KeyCode::Up => {
                if self.blog_index > 0 {
                    self.blog_index -= 1;
                    self.trigger_transition();
                }
            }
            KeyCode::Right | KeyCode::Down => {
                if self.blog_index < BLOG_ENTRIES.len() - 1 {
                    self.blog_index += 1;
                    self.trigger_transition();
                }
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
        let h_margin = if full_area.width < 60 { 1 } else { (full_area.width / 10).max(2) };
        let v_margin = if full_area.height < 30 { 0 } else { (full_area.height / 16).max(1) };

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

        // Compute individual tab click areas from the Tabs widget layout.
        // Each tab renders as: padding(1) + " title " + padding(1), with " │ " dividers.
        let divider_width: u16 = 3;
        let tab_padding: u16 = 2;
        let inner_x = area.x + 1;
        let tab_row = area.y + 1;
        self.tab_rects.clear();
        let mut pos = inner_x;
        for p in &Page::ALL {
            let title_len = p.title().len() as u16 + 2;
            let total = title_len + tab_padding;
            self.tab_rects.push(Rect::new(pos, tab_row, total, 1));
            pos += total + divider_width;
        }

        let titles: Vec<Line> = Page::ALL
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let hovered = self.tab_rects.get(i).is_some_and(|r| self.is_hovered(*r));
                let is_selected = self.page.index() == i;
                let fg = if is_selected {
                    Color::Rgb(230, 232, 240)
                } else if hovered {
                    Color::Rgb(255, 255, 255)
                } else {
                    Color::Rgb(140, 145, 155)
                };
                let style = if hovered && !is_selected {
                    Style::default().fg(fg).bold().add_modifier(Modifier::UNDERLINED)
                } else {
                    Style::default().fg(fg)
                };
                Line::from(vec![
                    Span::styled(" ", Style::default()),
                    Span::styled(p.title(), style),
                    Span::styled(" ", Style::default()),
                ])
            })
            .collect();

        let tabs = Tabs::new(titles)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Color::Rgb(55, 60, 70))
                    .title(" GRIFT.RS ")
                    .title_style(Style::default().fg(Color::Rgb(207, 181, 59)).bold()),
            )
            .select(self.page.index())
            .style(Style::default().fg(Color::Rgb(100, 105, 115)))
            .highlight_style(
                Style::default()
                    .fg(Color::Rgb(230, 232, 240))
                    .bold()
                    .add_modifier(Modifier::REVERSED),
            )
            .divider(" │ ");

        frame.render_widget(tabs, area);
    }

    fn render_home(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(55, 60, 70))
            .title_bottom(
                Line::from("│ swipe or click tabs to navigate │")
                    .alignment(Alignment::Right)
                    .style(Style::default().fg(Color::Rgb(55, 60, 70))),
            );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let is_narrow = inner.width < 50;
        let banner_height = BANNER.lines().count() as u16;

        if is_narrow {
            // Vertical layout for phones — skip banner, show all info sections stacked
            let [about_area, lisp_area, vau_area, rust_area, gsc_area] = Layout::vertical([
                Constraint::Length(4),
                Constraint::Length(6),
                Constraint::Length(7),
                Constraint::Length(6),
                Constraint::Min(5),
            ])
            .areas(inner);

            // About section
            let about = Paragraph::new(ABOUT)
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(Color::Rgb(140, 145, 155)))
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(Color::Rgb(40, 44, 52))
                        .title(" About ".bold().fg(Color::Rgb(200, 200, 210))),
                );
            frame.render_widget(about, about_area);

            // Lisp info
            let lisp = Paragraph::new(LISP_INFO)
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(Color::Rgb(170, 175, 185)))
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(Color::Rgb(40, 44, 52))
                        .title(" Lisp ".bold().fg(Color::Rgb(207, 181, 59))),
                );
            frame.render_widget(lisp, lisp_area);

            // Vau calculus info
            let vau = Paragraph::new(VAU_INFO)
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(Color::Rgb(170, 175, 185)))
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(Color::Rgb(40, 44, 52))
                        .title(" Vau Calculus ".bold().fg(Color::Rgb(184, 115, 51))),
                );
            frame.render_widget(vau, vau_area);

            // Rust info
            let rust = Paragraph::new(RUST_INFO)
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(Color::Rgb(170, 175, 185)))
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(Color::Rgb(40, 44, 52))
                        .title(" Rust ".bold().fg(Color::Rgb(222, 165, 132))),
                );
            frame.render_widget(rust, rust_area);

            // gold.silver.copper info
            let gsc = Paragraph::new(GSC_INFO)
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(Color::Rgb(170, 175, 185)))
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(Color::Rgb(40, 44, 52))
                        .title(" gold.silver.copper ".bold().fg(Color::Rgb(207, 181, 59))),
                );
            frame.render_widget(gsc, gsc_area);
        } else {
            // Wide layout — banner + description + two-column info grid
            let desc_lines = DESCRIPTION.lines().count() as u16;

            let [banner_area, desc_area, info_area, about_area] = Layout::vertical([
                Constraint::Length(banner_height),
                Constraint::Length(desc_lines + 2),
                Constraint::Min(6),
                Constraint::Length(4),
            ])
            .areas(inner);

            // Banner
            let banner = Paragraph::new(BANNER)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Rgb(200, 200, 210)).bold());
            frame.render_widget(banner, banner_area);
            self.banner_area = banner_area;

            // Grift description
            let desc = Paragraph::new(DESCRIPTION)
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(Color::Rgb(170, 175, 185)))
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(Color::Rgb(40, 44, 52))
                        .title(" Grift ".bold().fg(Color::Rgb(184, 115, 51))),
                );
            frame.render_widget(desc, desc_area);

            // Two-column info grid
            let [left_col, right_col] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(info_area);
            let [lisp_area, vau_area] =
                Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(left_col);
            let [rust_area, gsc_area] =
                Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(right_col);

            let lisp = Paragraph::new(LISP_INFO)
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(Color::Rgb(170, 175, 185)))
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(Color::Rgb(40, 44, 52))
                        .title(" Lisp ".bold().fg(Color::Rgb(207, 181, 59))),
                );
            frame.render_widget(lisp, lisp_area);

            let vau = Paragraph::new(VAU_INFO)
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(Color::Rgb(170, 175, 185)))
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(Color::Rgb(40, 44, 52))
                        .title(" Vau Calculus ".bold().fg(Color::Rgb(184, 115, 51))),
                );
            frame.render_widget(vau, vau_area);

            let rust = Paragraph::new(RUST_INFO)
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(Color::Rgb(170, 175, 185)))
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(Color::Rgb(40, 44, 52))
                        .title(" Rust ".bold().fg(Color::Rgb(222, 165, 132))),
                );
            frame.render_widget(rust, rust_area);

            let gsc = Paragraph::new(GSC_INFO)
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(Color::Rgb(170, 175, 185)))
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(Color::Rgb(40, 44, 52))
                        .title(" gold.silver.copper ".bold().fg(Color::Rgb(207, 181, 59))),
                );
            frame.render_widget(gsc, gsc_area);

            // About section at bottom
            let about = Paragraph::new(ABOUT)
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(Color::Rgb(140, 145, 155)))
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(Color::Rgb(40, 44, 52))
                        .title(" About ".bold().fg(Color::Rgb(200, 200, 210))),
                );
            frame.render_widget(about, about_area);
        }
    }

    fn render_repl(&self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(55, 60, 70))
            .title(" Grift REPL ".bold().fg(Color::Rgb(184, 115, 51)))
            .title_bottom(
                Line::from("│ Type expressions and press Enter to evaluate │")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::Rgb(55, 60, 70))),
            );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let [history_area, input_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).areas(inner);

        // History
        let mut history_lines: Vec<Line> = Vec::new();
        for (input, output) in &self.repl_history {
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
            .scroll((scroll as u16, 0))
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Color::Rgb(40, 44, 52))
                    .title(" Output ".fg(Color::Rgb(160, 165, 175))),
            );
        frame.render_widget(history, history_area);

        // Input
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
        let doc_content = match self.doc_page {
            0 => DOC_BASICS,
            1 => DOC_FORMS,
            2 => DOC_ADVANCED,
            _ => DOC_BASICS,
        };

        let doc_titles = ["Basics", "Forms", "Advanced"];

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(55, 60, 70))
            .title(
                format!(" Documentation: {} ", doc_titles[self.doc_page])
                    .bold()
                    .fg(Color::Rgb(200, 200, 210)),
            );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Split inner into doc content and navigation buttons
        let [doc_area, nav_bar] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).areas(inner);

        // Render doc content with syntax highlighting
        let lines: Vec<Line> = doc_content
            .lines()
            .map(|line| {
                if line.starts_with("  (") || line.starts_with("    (") {
                    if let Some((code, comment)) = line.split_once(';') {
                        if let Some((expr, result)) = code.split_once("=>") {
                            Line::from(vec![
                                Span::styled(expr, Style::default().fg(Color::Rgb(200, 200, 210))),
                                Span::styled("=>", Style::default().fg(Color::Rgb(80, 85, 95))),
                                Span::styled(
                                    result,
                                    Style::default().fg(Color::Rgb(160, 165, 175)),
                                ),
                                Span::styled(
                                    format!(";{comment}"),
                                    Style::default().fg(Color::Rgb(75, 80, 90)),
                                ),
                            ])
                        } else {
                            Line::from(vec![
                                Span::styled(code, Style::default().fg(Color::Rgb(200, 200, 210))),
                                Span::styled(
                                    format!(";{comment}"),
                                    Style::default().fg(Color::Rgb(75, 80, 90)),
                                ),
                            ])
                        }
                    } else if let Some((expr, result)) = line.split_once("=>") {
                        Line::from(vec![
                            Span::styled(expr, Style::default().fg(Color::Rgb(200, 200, 210))),
                            Span::styled("=>", Style::default().fg(Color::Rgb(80, 85, 95))),
                            Span::styled(result, Style::default().fg(Color::Rgb(160, 165, 175))),
                        ])
                    } else {
                        Line::styled(line, Style::default().fg(Color::Rgb(200, 200, 210)))
                    }
                } else if line.starts_with("  ") && !line.trim().is_empty() {
                    if let Some((code, comment)) = line.split_once(';') {
                        Line::from(vec![
                            Span::styled(code, Style::default().fg(Color::Rgb(200, 200, 210))),
                            Span::styled(
                                format!(";{comment}"),
                                Style::default().fg(Color::Rgb(75, 80, 90)),
                            ),
                        ])
                    } else if let Some((expr, result)) = line.split_once("=>") {
                        Line::from(vec![
                            Span::styled(expr, Style::default().fg(Color::Rgb(200, 200, 210))),
                            Span::styled("=>", Style::default().fg(Color::Rgb(80, 85, 95))),
                            Span::styled(result, Style::default().fg(Color::Rgb(160, 165, 175))),
                        ])
                    } else {
                        Line::styled(line, Style::default().fg(Color::Rgb(200, 200, 210)))
                    }
                } else if line.contains('─') {
                    Line::styled(line, Style::default().fg(Color::Rgb(140, 145, 155)))
                } else if !line.trim().is_empty() {
                    Line::styled(
                        line,
                        Style::default().fg(Color::Rgb(220, 225, 235)).bold(),
                    )
                } else {
                    Line::from("")
                }
            })
            .collect();

        let doc = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false });
        frame.render_widget(doc, doc_area);

        // Navigation buttons — naturally highlighted, extra effect on hover
        let [prev_area, info_area, next_area] = Layout::horizontal([
            Constraint::Length(12),
            Constraint::Min(1),
            Constraint::Length(12),
        ])
        .areas(nav_bar);

        self.doc_nav_prev = prev_area;
        self.doc_nav_next = next_area;

        let prev_hovered = self.doc_page > 0 && self.is_hovered(prev_area);
        let next_hovered = self.doc_page < 2 && self.is_hovered(next_area);

        let prev_style = if prev_hovered {
            Style::default()
                .fg(Color::Rgb(255, 255, 255))
                .bold()
                .add_modifier(Modifier::REVERSED)
        } else if self.doc_page > 0 {
            Style::default().fg(Color::Rgb(200, 200, 210)).bold()
        } else {
            Style::default().fg(Color::Rgb(45, 48, 56))
        };
        let next_style = if next_hovered {
            Style::default()
                .fg(Color::Rgb(255, 255, 255))
                .bold()
                .add_modifier(Modifier::REVERSED)
        } else if self.doc_page < 2 {
            Style::default().fg(Color::Rgb(200, 200, 210)).bold()
        } else {
            Style::default().fg(Color::Rgb(45, 48, 56))
        };

        let prev_border = if prev_hovered {
            Color::Rgb(200, 200, 210)
        } else if self.doc_page > 0 {
            Color::Rgb(80, 85, 95)
        } else {
            Color::Rgb(40, 44, 52)
        };
        let next_border = if next_hovered {
            Color::Rgb(200, 200, 210)
        } else if self.doc_page < 2 {
            Color::Rgb(80, 85, 95)
        } else {
            Color::Rgb(40, 44, 52)
        };

        frame.render_widget(
            Paragraph::new(" [< Prev]").style(prev_style).block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(prev_border),
            ),
            prev_area,
        );
        frame.render_widget(
            Paragraph::new(format!(" Page {}/{} ", self.doc_page + 1, doc_titles.len()))
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Rgb(100, 105, 115)))
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(Color::Rgb(40, 44, 52)),
                ),
            info_area,
        );
        frame.render_widget(
            Paragraph::new(" [Next >]").style(next_style).block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(next_border),
            ),
            next_area,
        );
    }

    fn render_blog(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(55, 60, 70))
            .title(" Blog ".bold().fg(Color::Rgb(200, 200, 210)))
            .title_bottom(
                Line::from("│ click a post or swipe │")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::Rgb(55, 60, 70))),
            );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let is_narrow = inner.width < 50;

        if is_narrow {
            // Vertical layout for phones — list on top, content below
            let [list_area, content_area] =
                Layout::vertical([Constraint::Length((BLOG_ENTRIES.len() as u16 * 2) + 2), Constraint::Min(1)]).areas(inner);

            // Blog list
            let list_block = Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Color::Rgb(40, 44, 52))
                .title(" Posts ".fg(Color::Rgb(200, 200, 210)));

            let list_inner = list_block.inner(list_area);

            self.blog_item_areas.clear();
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
            frame.render_widget(list, list_area);

            // Blog content
            if let Some((title, date, content)) = BLOG_ENTRIES.get(self.blog_index) {
                let mut lines = vec![
                    Line::styled(*title, Style::default().fg(Color::Rgb(220, 225, 235)).bold()),
                    Line::styled(*date, Style::default().fg(Color::Rgb(75, 80, 90))),
                    Line::from(""),
                ];
                for line in content.lines() {
                    lines.push(Line::styled(line, Style::default().fg(Color::Rgb(170, 175, 185))));
                }
                let blog = Paragraph::new(Text::from(lines))
                    .wrap(Wrap { trim: false })
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .border_style(Color::Rgb(40, 44, 52)),
                    );
                frame.render_widget(blog, content_area);
            }
        } else {
            // Wide layout — side-by-side
            let [list_area, content_area] =
                Layout::horizontal([Constraint::Length(35), Constraint::Min(1)]).areas(inner);

            // Blog list - track clickable areas
            let list_block = Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Color::Rgb(40, 44, 52))
                .title(" Posts ".fg(Color::Rgb(200, 200, 210)));

            let list_inner = list_block.inner(list_area);

            self.blog_item_areas.clear();
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
                    let marker = if i == self.blog_index {
                        "> "
                    } else if hovered {
                        "~ "
                    } else {
                        "  "
                    };
                    ListItem::new(vec![
                        Line::from(format!("{marker}{title}")).style(style),
                        Line::from(format!("  {date}"))
                            .style(Style::default().fg(Color::Rgb(75, 80, 90))),
                    ])
                })
                .collect();

            let list = List::new(items).block(list_block);
            frame.render_widget(list, list_area);

            // Blog content
            if let Some((title, date, content)) = BLOG_ENTRIES.get(self.blog_index) {
                let mut lines = vec![
                    Line::styled(
                        *title,
                        Style::default().fg(Color::Rgb(220, 225, 235)).bold(),
                    ),
                    Line::styled(*date, Style::default().fg(Color::Rgb(75, 80, 90))),
                    Line::styled(
                        "─".repeat(content_area.width.saturating_sub(4) as usize),
                        Style::default().fg(Color::Rgb(40, 44, 52)),
                    ),
                    Line::from(""),
                ];
                for line in content.lines() {
                    lines.push(Line::styled(
                        line,
                        Style::default().fg(Color::Rgb(170, 175, 185)),
                    ));
                }

                let blog = Paragraph::new(Text::from(lines))
                    .wrap(Wrap { trim: false })
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .border_style(Color::Rgb(40, 44, 52)),
                    );
                frame.render_widget(blog, content_area);
            }
        }
    }

    fn render_links(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(55, 60, 70))
            .title(" Links ".bold().fg(Color::Rgb(200, 200, 210)))
            .title_bottom(
                Line::from("│ click a link to open │")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::Rgb(55, 60, 70))),
            );

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let [links_area, info_area] =
            Layout::vertical([Constraint::Length(LINKS.len() as u16 + 2), Constraint::Min(1)])
                .areas(inner);

        // Links with hyperlinks
        let links_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(40, 44, 52))
            .title(" Repositories & Resources ".fg(Color::Rgb(200, 200, 210)));

        let links_inner = links_block.inner(links_area);
        frame.render_widget(links_block, links_area);

        self.link_areas.clear();
        for (i, (label, _url)) in LINKS.iter().enumerate() {
            let link_area = Rect::new(links_inner.x, links_inner.y + i as u16, links_inner.width, 1);
            if link_area.y < links_inner.bottom() {
                let hovered = self.is_hovered(link_area);
                let style = if hovered {
                    Style::default()
                        .fg(Color::Rgb(255, 255, 255))
                        .bold()
                        .add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                        .fg(Color::Rgb(160, 175, 195))
                };
                let marker = if hovered { "> " } else { "  " };
                let text = Paragraph::new(format!("{marker}{label}")).style(style);
                frame.render_widget(text, link_area);
                self.link_areas.push(link_area);
            }
        }

        // Info section — technical details about the site
        let info_text = Text::from(vec![
            Line::from(""),
            Line::from(vec![
                "  gold.silver.copper ".fg(Color::Rgb(207, 181, 59)).bold(),
                "— Software developer & language designer"
                    .fg(Color::Rgb(140, 145, 155)),
            ]),
            Line::from(""),
            Line::from(vec![
                "  Grift ".fg(Color::Rgb(184, 115, 51)).bold(),
                "– A minimalistic Lisp implementing vau calculus"
                    .fg(Color::Rgb(140, 145, 155)),
            ]),
            Line::from(""),
            Line::from(
                "  This website is entirely rendered as a terminal UI in your browser."
                    .fg(Color::Rgb(100, 105, 115)),
            ),
            Line::from(
                "  Powered by Ratzilla + TachyonFX + WebAssembly."
                    .fg(Color::Rgb(100, 105, 115)),
            ),
        ]);
        frame.render_widget(
            Paragraph::new(info_text)
                .wrap(Wrap { trim: false })
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(Color::Rgb(40, 44, 52))
                        .title(" Info ".fg(Color::Rgb(200, 200, 210))),
                ),
            info_area,
        );
    }
}

fn open_url(url: &str) {
    let _ = ratzilla::utils::open_url(url, true);
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
            });
            $terminal.on_mouse_event({
                let app = $app.clone();
                move |mouse_event| { app.borrow_mut().handle_mouse_event(mouse_event); }
            });
            $terminal.draw_web({
                let app = $app.clone();
                move |frame| { app.borrow_mut().draw(frame); }
            });
        }};
    }

    let options = WebGl2BackendOptions::new()
        .enable_mouse_selection_with_mode(Default::default());
    let backend = WebGl2Backend::new_with_options(options).expect("failed to create WebGL2 backend");
    let terminal = ratzilla::ratatui::Terminal::new(backend)?;
    setup_terminal!(terminal, app);

    Ok(())
}
