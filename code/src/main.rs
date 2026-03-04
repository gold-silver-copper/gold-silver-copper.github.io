use std::cell::RefCell;
use std::rc::Rc;

use grift::Lisp;
use ratzilla::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratzilla::ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratzilla::ratatui::style::{Color, Modifier, Style, Stylize};
use ratzilla::ratatui::text::{Line, Span, Text};
use ratzilla::ratatui::widgets::{Block, BorderType, Paragraph, Wrap};
use ratzilla::ratatui::Frame;
use ratzilla::backend::webgl2::WebGl2BackendOptions;
use ratzilla::WebGl2Backend;
use ratzilla::WebRenderer;
use unicode_width::UnicodeWidthChar;

use tachyonfx::fx::{self};
use tachyonfx::dsl::EffectDsl;
use tachyonfx::{CellFilter, Duration, Effect, EffectRenderer, EffectTimer, Interpolation, Motion};

const TRAIL_INITIAL_INTENSITY: u8 = 200;
const MAX_TRAIL_LENGTH: usize = 30;
const TRAIL_FADE_RATE: u8 = 8;
const CURSOR_BLINK_RATE: u64 = 60;

// Unified layout: always use compact mobile-style sizing
const MARGIN_DIVISOR: u16 = 16;

const DESCRIPTION: &str = "\
A Kernel-style Lisp built in Rust where everything is first-class. Operatives (fexprs) subsume both functions and macros — receiving arguments unevaluated with access to the caller's environment. no_std, no_alloc, #![forbid(unsafe_code)], compiles to WASM. Grift is designed for minimalism and correctness: the entire evaluator fits in a single file with zero dependencies on heap allocation or unsafe code.";

const VAU_INFO: &str = "\
Grift implements vau calculus: first-class operatives that receive their operands unevaluated alongside the dynamic environment. This single primitive replaces the function/macro split entirely. User-defined operatives have the same power as built-in forms — define!, if, and quote are all expressible in user space. Vau calculus was introduced by John Shutt in his 2010 PhD thesis as a cleaner foundation for Lisp semantics.";

const FIRST_CLASS_INFO: &str = "\
Environments, continuations, operatives, and combiners are all first-class values. Operatives close over their static environment and capture the caller's dynamic environment at each call site. This enables reflective towers, hygienic binding constructs, and arbitrary evaluation strategies — without special-casing. First-class environments mean you can pass, return, and inspect environments just like any other value in the language.";

const IMPL_INFO: &str = "\
Written in pure Rust: arena-allocated with const-generic capacity, tail-call optimized, mark-and-sweep GC, zero unsafe code. Runs on bare-metal embedded targets and compiles to WebAssembly. This entire site is a Rust TUI rendered to canvas via WASM. The arena allocator uses a fixed-size array with const generics so the capacity is determined at compile time with no runtime overhead.";

const LINKS: &[(&str, &str, &str)] = &[
    (
        "GitHub (gold-silver-copper)",
        "https://github.com/gold-silver-copper",
        "Main GitHub profile — hosts open-source Rust and Lisp projects including grift and this website.",
    ),
    (
        "GitHub (grift)",
        "https://github.com/skyfskyf/grift",
        "The Grift language repository — a no_std, no_alloc Kernel-style Lisp that powers this site's REPL.",
    ),
    (
        "GitHub (grift-site)",
        "https://github.com/skyfskyf/grift-site",
        "Source code for this very website — a terminal UI compiled to WASM and rendered to canvas.",
    ),
    (
        "Ratzilla – Terminal web apps with Rust + WASM",
        "https://github.com/ratatui/ratzilla",
        "The framework that renders this terminal UI in your browser via WebGL2 — built on top of ratatui.",
    ),
    (
        "Ratatui – Terminal UI framework",
        "https://github.com/ratatui/ratatui",
        "The Rust TUI framework providing all the widgets, layout, and text rendering used throughout this site.",
    ),
    (
        "TachyonFX – Shader-like effects for TUIs",
        "https://github.com/ratatui/tachyonfx",
        "Provides the animated transitions, hover effects, and background animations you see across every page.",
    ),
    (
        "WebAssembly",
        "https://webassembly.org",
        "The compilation target that makes this Rust application run natively in your browser at near-native speed.",
    ),
    (
        "Rust Programming Language",
        "https://www.rust-lang.org",
        "The language this entire site is written in — zero JavaScript frameworks, just safe Rust compiled to WASM.",
    ),
    (
        "crates.io – Rust Package Registry",
        "https://crates.io",
        "Where Grift, TachyonFX, and other Rust dependencies used by this project are published.",
    ),
    (
        "Kernel Language (vau calculus)",
        "https://web.cs.wpi.edu/~jshutt/kernel.html",
        "The theoretical foundation for Grift — John Shutt's Kernel language where operatives replace macros.",
    ),
    (
        "John Shutt – Vau Calculus Thesis",
        "https://web.cs.wpi.edu/~jshutt/dissertation/etd-090110-124904.pdf",
        "The 2010 PhD thesis that introduced vau calculus as a cleaner semantic foundation for Lisp.",
    ),
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

const DOC_ENVIRONMENTS: &str = "\
Environments & Evaluation\n\
─────────────────────────\n\
\n\
Environments are first-class in Grift:\n\
  (get-current-environment)  => <environment>\n\
  (make-environment)         => <empty-env>\n\
  (eval expr env)            => evaluate expr in env\n\
\n\
Operatives receive the dynamic environment:\n\
  ($vau (x) e (eval x e))   ; like lambda\n\
  (wrap ($vau (x) #ignore x)) ; applicative from operative\n\
\n\
The evaluator:\n\
  1. Symbols are looked up in the current environment\n\
  2. Pairs: evaluate the operator, then combine\n\
  3. Operatives receive operands unevaluated\n\
  4. Applicatives evaluate operands first, then call\n\
\n\
Tail-call optimization:\n\
  Grift optimizes tail positions so recursive functions\n\
  run in constant stack space. This applies to if, cond,\n\
  begin, let, and operative/applicative bodies.";

const DOC_ERRORS: &str = "\
Error Handling & Debugging\n\
──────────────────────────\n\
\n\
Grift reports errors as readable messages:\n\
  (/ 1 0)               => Error: DivisionByZero\n\
  (car 42)              => Error: TypeMismatch\n\
  undefined-sym          => Error: UnboundSymbol\n\
\n\
Common errors:\n\
  TypeMismatch    — wrong argument type\n\
  ArityMismatch   — wrong number of arguments\n\
  UnboundSymbol   — symbol not defined in scope\n\
  DivisionByZero  — division by zero\n\
  ArenaFull       — arena capacity exceeded\n\
\n\
Debugging tips:\n\
  1. Check types with predicates: number?, pair?, string?\n\
  2. Inspect environments with get-current-environment\n\
  3. Use begin to sequence debug prints\n\
  4. Break complex expressions into smaller define! steps";

// ---------------------------------------------------------------------------
// Expanded Effects DSL Showcase
// ---------------------------------------------------------------------------
// Each entry: (category, title, DSL expression string).
// The DSL expressions are compiled at runtime by tachyonfx::dsl::EffectDsl.
// Effects are wrapped with repeating(ping_pong(...)) so they loop forever.
// ---------------------------------------------------------------------------

struct DslShowcaseEntry {
    category: &'static str,
    title: &'static str,
    dsl: &'static str,
}

const DSL_SHOWCASE: &[DslShowcaseEntry] = &[
    // ── Dissolve / Coalesce ──────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Dissolve & Coalesce",
        title: "dissolve",
        dsl: "fx::dissolve(2000)",
    },
    DslShowcaseEntry {
        category: "Dissolve & Coalesce",
        title: "coalesce",
        dsl: "fx::coalesce(2000)",
    },
    DslShowcaseEntry {
        category: "Dissolve & Coalesce",
        title: "dissolve (QuadOut)",
        dsl: "fx::dissolve((2500, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Dissolve & Coalesce",
        title: "coalesce (SineOut)",
        dsl: "fx::coalesce((2500, SineOut))",
    },
    DslShowcaseEntry {
        category: "Dissolve & Coalesce",
        title: "dissolve (BounceOut)",
        dsl: "fx::dissolve((3000, BounceOut))",
    },
    DslShowcaseEntry {
        category: "Dissolve & Coalesce",
        title: "dissolve (CubicInOut)",
        dsl: "fx::dissolve((2500, CubicInOut))",
    },
    DslShowcaseEntry {
        category: "Dissolve & Coalesce",
        title: "dissolve_to amber",
        dsl: "fx::dissolve_to(Color::Rgb(207, 181, 59), (2500, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Dissolve & Coalesce",
        title: "coalesce_from teal",
        dsl: "fx::coalesce_from(Color::Rgb(0, 180, 180), (2500, CubicOut))",
    },

    // ── Slide / Sweep ────────────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Slide & Sweep",
        title: "sweep_in L→R",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sweep_in(Motion::LeftToRight, 10, 3, bg, (3000, QuadOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Slide & Sweep",
        title: "sweep_in R→L",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sweep_in(Motion::RightToLeft, 10, 3, bg, (3000, QuadOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Slide & Sweep",
        title: "sweep_in U→D",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sweep_in(Motion::UpToDown, 8, 2, bg, (3000, CubicOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Slide & Sweep",
        title: "sweep_in D→U",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sweep_in(Motion::DownToUp, 8, 2, bg, (3000, CubicOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Slide & Sweep",
        title: "sweep_out L→R",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sweep_out(Motion::LeftToRight, 10, 3, bg, (3000, QuadOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Slide & Sweep",
        title: "slide_in L→R",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::slide_in(Motion::LeftToRight, 8, 3, bg, (3000, CubicOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Slide & Sweep",
        title: "slide_in U→D",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::slide_in(Motion::UpToDown, 8, 3, bg, (3000, CubicOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Slide & Sweep",
        title: "slide_out R→L",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::slide_out(Motion::RightToLeft, 8, 3, bg, (3000, QuadOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Slide & Sweep",
        title: "sweep_in wide L→R",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sweep_in(Motion::LeftToRight, 20, 6, bg, (4000, SineOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Slide & Sweep",
        title: "slide_in narrow D→U",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::slide_in(Motion::DownToUp, 4, 1, bg, (2500, QuadOut))
        "#,
    },

    // ── Color Fading ─────────────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Color Fading",
        title: "fade_from black",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::fade_from(bg, bg, (3000, CubicOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Color Fading",
        title: "fade_to_fg red",
        dsl: "fx::fade_to_fg(Color::Red, (2500, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Color Fading",
        title: "fade_to_fg blue",
        dsl: "fx::fade_to_fg(Color::Blue, (2500, SineOut))",
    },
    DslShowcaseEntry {
        category: "Color Fading",
        title: "fade_to_fg green",
        dsl: "fx::fade_to_fg(Color::Green, (2500, CubicOut))",
    },
    DslShowcaseEntry {
        category: "Color Fading",
        title: "fade_to_fg amber",
        dsl: "fx::fade_to_fg(Color::Rgb(207, 181, 59), (2500, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Color Fading",
        title: "fade_from_fg cyan",
        dsl: "fx::fade_from_fg(Color::Cyan, (2500, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Color Fading",
        title: "fade_from_fg magenta",
        dsl: "fx::fade_from_fg(Color::Magenta, (3000, SineOut))",
    },
    DslShowcaseEntry {
        category: "Color Fading",
        title: "fade_to pink→amber",
        dsl: r#"fx::fade_to(Color::Rgb(255, 105, 180), Color::Rgb(207, 181, 59), (3000, CubicOut))"#,
    },
    DslShowcaseEntry {
        category: "Color Fading",
        title: "fade_from deep blue",
        dsl: r#"fx::fade_from(Color::Rgb(0, 30, 90), Color::Rgb(0, 30, 90), (3500, SineOut))"#,
    },
    DslShowcaseEntry {
        category: "Color Fading",
        title: "fade_to_fg orange (BounceOut)",
        dsl: "fx::fade_to_fg(Color::Rgb(255, 140, 0), (3000, BounceOut))",
    },

    // ── HSL Manipulation ─────────────────────────────────────────────────
    DslShowcaseEntry {
        category: "HSL Manipulation",
        title: "hsl_shift_fg warm",
        dsl: "fx::hsl_shift_fg([30.0, 20.0, 25.0], (3000, SineOut))",
    },
    DslShowcaseEntry {
        category: "HSL Manipulation",
        title: "hsl_shift_fg cool",
        dsl: "fx::hsl_shift_fg([-40.0, 15.0, -10.0], (3000, QuadOut))",
    },
    DslShowcaseEntry {
        category: "HSL Manipulation",
        title: "hsl_shift_fg vibrant",
        dsl: "fx::hsl_shift_fg([60.0, 40.0, 30.0], (3500, CubicOut))",
    },
    DslShowcaseEntry {
        category: "HSL Manipulation",
        title: "hsl_shift_fg pastel",
        dsl: "fx::hsl_shift_fg([20.0, -30.0, 40.0], (3000, SineOut))",
    },
    DslShowcaseEntry {
        category: "HSL Manipulation",
        title: "hsl_shift_fg neon",
        dsl: "fx::hsl_shift_fg([90.0, 50.0, 20.0], (4000, QuadInOut))",
    },
    DslShowcaseEntry {
        category: "HSL Manipulation",
        title: "hsl_shift full spectrum",
        dsl: "fx::hsl_shift_fg([180.0, 0.0, 0.0], (5000, Linear))",
    },
    DslShowcaseEntry {
        category: "HSL Manipulation",
        title: "saturate_fg",
        dsl: "fx::saturate_fg(50.0, (3000, QuadOut))",
    },
    DslShowcaseEntry {
        category: "HSL Manipulation",
        title: "lighten_fg",
        dsl: "fx::lighten_fg(40.0, (3000, SineOut))",
    },
    DslShowcaseEntry {
        category: "HSL Manipulation",
        title: "darken_fg",
        dsl: "fx::darken_fg(40.0, (3000, QuadOut))",
    },
    DslShowcaseEntry {
        category: "HSL Manipulation",
        title: "hsl_shift_fg muted sunset",
        dsl: "fx::hsl_shift_fg([45.0, -20.0, 15.0], (3500, CubicOut))",
    },

    // ── Paint Effects ────────────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Paint Effects",
        title: "paint_fg gold",
        dsl: "fx::paint_fg(Color::Rgb(207, 181, 59), (2500, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Paint Effects",
        title: "paint_fg copper",
        dsl: "fx::paint_fg(Color::Rgb(184, 115, 51), (2500, CubicOut))",
    },
    DslShowcaseEntry {
        category: "Paint Effects",
        title: "paint_fg silver",
        dsl: "fx::paint_fg(Color::Rgb(192, 192, 192), (2500, SineOut))",
    },
    DslShowcaseEntry {
        category: "Paint Effects",
        title: "paint_fg cyan",
        dsl: "fx::paint_fg(Color::Cyan, (2500, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Paint Effects",
        title: "paint_fg hot pink",
        dsl: "fx::paint_fg(Color::Rgb(255, 105, 180), (2500, CubicOut))",
    },

    // ── Explosion & Stretch ──────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Explosion & Motion",
        title: "explode",
        dsl: "fx::explode((3000, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Explosion & Motion",
        title: "stretch L→R",
        dsl: "fx::stretch(Motion::LeftToRight, (3000, CubicOut))",
    },
    DslShowcaseEntry {
        category: "Explosion & Motion",
        title: "stretch U→D",
        dsl: "fx::stretch(Motion::UpToDown, (3000, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Explosion & Motion",
        title: "expand L→R",
        dsl: "fx::expand(Motion::LeftToRight, (3000, CubicOut))",
    },
    DslShowcaseEntry {
        category: "Explosion & Motion",
        title: "expand U→D",
        dsl: "fx::expand(Motion::UpToDown, (3000, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Explosion & Motion",
        title: "translate",
        dsl: "fx::translate(3, 1, (2500, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Explosion & Motion",
        title: "translate reverse",
        dsl: "fx::translate(-3, -1, (2500, CubicOut))",
    },
    DslShowcaseEntry {
        category: "Explosion & Motion",
        title: "explode (BounceOut)",
        dsl: "fx::explode((3500, BounceOut))",
    },

    // ── Sequences & Compositions ─────────────────────────────────────────
    DslShowcaseEntry {
        category: "Compositions",
        title: "sequence: dissolve → coalesce",
        dsl: r#"
            fx::sequence(&[
                fx::dissolve(1500),
                fx::coalesce(1500)
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Compositions",
        title: "sequence: fade colors",
        dsl: r#"
            fx::sequence(&[
                fx::fade_to_fg(Color::Red, 1000),
                fx::fade_to_fg(Color::Blue, 1000),
                fx::fade_to_fg(Color::Green, 1000)
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Compositions",
        title: "parallel: dissolve + hsl",
        dsl: r#"
            fx::parallel(&[
                fx::dissolve((3000, QuadOut)),
                fx::hsl_shift_fg([60.0, 30.0, 20.0], (3000, SineOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Compositions",
        title: "sequence: triple sweep",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sequence(&[
                fx::sweep_in(Motion::LeftToRight, 10, 3, bg, (1200, QuadOut)),
                fx::sweep_in(Motion::RightToLeft, 10, 3, bg, (1200, QuadOut)),
                fx::sweep_in(Motion::UpToDown, 8, 2, bg, (1200, CubicOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Compositions",
        title: "parallel: fade + sweep",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::parallel(&[
                fx::fade_to_fg(Color::Rgb(207, 181, 59), (3000, QuadOut)),
                fx::sweep_in(Motion::LeftToRight, 15, 5, bg, (3000, SineOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Compositions",
        title: "sequence: paint chain",
        dsl: r#"
            fx::sequence(&[
                fx::paint_fg(Color::Red, 800),
                fx::paint_fg(Color::Rgb(255, 165, 0), 800),
                fx::paint_fg(Color::Yellow, 800),
                fx::paint_fg(Color::Green, 800)
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Compositions",
        title: "parallel: triple shift",
        dsl: r#"
            fx::parallel(&[
                fx::hsl_shift_fg([120.0, 0.0, 0.0], (4000, Linear)),
                fx::lighten_fg(20.0, (4000, SineOut)),
                fx::saturate_fg(30.0, (4000, QuadOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Compositions",
        title: "sequence: slide bounce",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sequence(&[
                fx::slide_in(Motion::LeftToRight, 8, 3, bg, (1500, BounceOut)),
                fx::slide_in(Motion::RightToLeft, 8, 3, bg, (1500, BounceOut))
            ])
        "#,
    },

    // ── With Patterns ────────────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Patterns",
        title: "dissolve + radial center",
        dsl: r#"
            fx::dissolve((3000, QuadOut))
                .with_pattern(RadialPattern::center())
        "#,
    },
    DslShowcaseEntry {
        category: "Patterns",
        title: "coalesce + radial center",
        dsl: r#"
            fx::coalesce((3000, CubicOut))
                .with_pattern(RadialPattern::center())
        "#,
    },
    DslShowcaseEntry {
        category: "Patterns",
        title: "dissolve + diamond",
        dsl: r#"
            fx::dissolve((3000, QuadOut))
                .with_pattern(DiamondPattern::center())
        "#,
    },
    DslShowcaseEntry {
        category: "Patterns",
        title: "coalesce + diamond",
        dsl: r#"
            fx::coalesce((3000, SineOut))
                .with_pattern(DiamondPattern::center())
        "#,
    },
    DslShowcaseEntry {
        category: "Patterns",
        title: "dissolve + spiral 4 arms",
        dsl: r#"
            fx::dissolve((4000, Linear))
                .with_pattern(SpiralPattern::center().with_arms(4))
        "#,
    },
    DslShowcaseEntry {
        category: "Patterns",
        title: "coalesce + spiral 6 arms",
        dsl: r#"
            fx::coalesce((4000, Linear))
                .with_pattern(SpiralPattern::center().with_arms(6))
        "#,
    },
    DslShowcaseEntry {
        category: "Patterns",
        title: "dissolve + diagonal TL→BR",
        dsl: r#"
            fx::dissolve((3000, QuadOut))
                .with_pattern(DiagonalPattern::top_left_to_bottom_right())
        "#,
    },
    DslShowcaseEntry {
        category: "Patterns",
        title: "dissolve + checkerboard",
        dsl: r#"
            fx::dissolve((3000, CubicOut))
                .with_pattern(CheckerboardPattern::default())
        "#,
    },
    DslShowcaseEntry {
        category: "Patterns",
        title: "dissolve + sweep pattern L→R",
        dsl: r#"
            fx::dissolve((3000, QuadOut))
                .with_pattern(SweepPattern::left_to_right())
        "#,
    },
    DslShowcaseEntry {
        category: "Patterns",
        title: "coalesce + inverted radial",
        dsl: r#"
            fx::coalesce((3000, SineOut))
                .with_pattern(InvertedPattern::new(RadialPattern::center()))
        "#,
    },
    DslShowcaseEntry {
        category: "Patterns",
        title: "dissolve + spiral wide",
        dsl: r#"
            fx::dissolve((4000, QuadOut))
                .with_pattern(
                    SpiralPattern::center()
                        .with_arms(3)
                        .with_transition_width(2.5)
                )
        "#,
    },
    DslShowcaseEntry {
        category: "Patterns",
        title: "dissolve + combined radial×diamond",
        dsl: r#"
            fx::dissolve((4000, SineOut))
                .with_pattern(CombinedPattern::multiply(
                    RadialPattern::center(),
                    DiamondPattern::center()
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Patterns",
        title: "coalesce + blend spiral↔radial",
        dsl: r#"
            fx::coalesce((4000, QuadOut))
                .with_pattern(BlendPattern::new(
                    SpiralPattern::center().with_arms(4),
                    RadialPattern::center()
                ))
        "#,
    },

    // ── Wave Patterns ────────────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Wave Patterns",
        title: "dissolve + sine wave",
        dsl: r#"
            fx::dissolve((4000, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(Oscillator::sin(2.0, 0.0, 1.0))
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Wave Patterns",
        title: "dissolve + cos wave",
        dsl: r#"
            fx::dissolve((4000, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(Oscillator::cos(0.0, 3.0, 0.5))
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Wave Patterns",
        title: "coalesce + triangle wave",
        dsl: r#"
            fx::coalesce((4000, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(Oscillator::triangle(2.0, 2.0, 0.8))
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Wave Patterns",
        title: "dissolve + sawtooth wave",
        dsl: r#"
            fx::dissolve((4000, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(Oscillator::sawtooth(3.0, 0.0, 1.0))
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Wave Patterns",
        title: "dissolve + modulated wave",
        dsl: r#"
            fx::dissolve((5000, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(
                        Oscillator::sin(2.0, 0.0, 1.0)
                            .modulated_by(Modulator::sin(1.0, 1.0, 0.25).intensity(0.5))
                    )
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Wave Patterns",
        title: "coalesce + multiplied waves",
        dsl: r#"
            fx::coalesce((5000, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(Oscillator::sin(2.0, 0.0, 1.0))
                        .multiply(Oscillator::cos(0.0, 3.0, 0.5))
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Wave Patterns",
        title: "dissolve + wave contrast",
        dsl: r#"
            fx::dissolve((4500, Linear))
                .with_pattern(
                    WavePattern::new(
                        WaveLayer::new(Oscillator::sin(3.0, 0.0, 0.8))
                            .amplitude(0.9)
                    ).with_contrast(3)
                )
        "#,
    },
    DslShowcaseEntry {
        category: "Wave Patterns",
        title: "dissolve + complex wave",
        dsl: r#"
            fx::dissolve((6000, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(Oscillator::sin(2.0, 0.0, 1.0))
                        .multiply(
                            Oscillator::cos(0.0, 3.0, 0.5)
                                .modulated_by(
                                    Modulator::sin(1.0, 1.0, 0.25)
                                        .intensity(0.5)
                                )
                        )
                        .amplitude(0.8)
                ))
        "#,
    },

    // ── Timing & Control ─────────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Timing & Control",
        title: "with_duration dissolve",
        dsl: "fx::with_duration(4000, fx::dissolve(2000))",
    },
    DslShowcaseEntry {
        category: "Timing & Control",
        title: "delay + coalesce",
        dsl: r#"
            fx::sequence(&[
                fx::sleep(500),
                fx::coalesce((2500, SineOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Timing & Control",
        title: "prolong_start dissolve",
        dsl: "fx::prolong_start(500, fx::dissolve((2000, QuadOut)))",
    },
    DslShowcaseEntry {
        category: "Timing & Control",
        title: "prolong_end coalesce",
        dsl: "fx::prolong_end(500, fx::coalesce((2000, SineOut)))",
    },
    DslShowcaseEntry {
        category: "Timing & Control",
        title: "sequence delayed sweeps",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sequence(&[
                fx::sweep_in(Motion::LeftToRight, 10, 3, bg, (1200, QuadOut)),
                fx::sleep(300),
                fx::sweep_in(Motion::RightToLeft, 10, 3, bg, (1200, QuadOut)),
                fx::sleep(300),
                fx::sweep_in(Motion::UpToDown, 8, 2, bg, (1200, CubicOut))
            ])
        "#,
    },

    // ── Cell Filters ─────────────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Cell Filters",
        title: "dissolve Text only",
        dsl: r#"
            fx::dissolve((2500, QuadOut))
                .with_filter(CellFilter::Text)
        "#,
    },
    DslShowcaseEntry {
        category: "Cell Filters",
        title: "fade_to_fg NonEmpty",
        dsl: r#"
            fx::fade_to_fg(Color::Rgb(207, 181, 59), (2500, SineOut))
                .with_filter(CellFilter::NonEmpty)
        "#,
    },
    DslShowcaseEntry {
        category: "Cell Filters",
        title: "hsl_shift Text filter",
        dsl: r#"
            fx::hsl_shift_fg([45.0, 25.0, 20.0], (3000, QuadOut))
                .with_filter(CellFilter::Text)
        "#,
    },

    // ── Interpolation Showcase ───────────────────────────────────────────
    DslShowcaseEntry {
        category: "Interpolation",
        title: "dissolve Linear",
        dsl: "fx::dissolve((3000, Linear))",
    },
    DslShowcaseEntry {
        category: "Interpolation",
        title: "dissolve QuadIn",
        dsl: "fx::dissolve((3000, QuadIn))",
    },
    DslShowcaseEntry {
        category: "Interpolation",
        title: "dissolve QuadOut",
        dsl: "fx::dissolve((3000, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Interpolation",
        title: "dissolve CubicIn",
        dsl: "fx::dissolve((3000, CubicIn))",
    },
    DslShowcaseEntry {
        category: "Interpolation",
        title: "dissolve CubicOut",
        dsl: "fx::dissolve((3000, CubicOut))",
    },
    DslShowcaseEntry {
        category: "Interpolation",
        title: "dissolve CubicInOut",
        dsl: "fx::dissolve((3000, CubicInOut))",
    },
    DslShowcaseEntry {
        category: "Interpolation",
        title: "dissolve SineIn",
        dsl: "fx::dissolve((3000, SineIn))",
    },
    DslShowcaseEntry {
        category: "Interpolation",
        title: "dissolve SineOut",
        dsl: "fx::dissolve((3000, SineOut))",
    },
    DslShowcaseEntry {
        category: "Interpolation",
        title: "dissolve BounceOut",
        dsl: "fx::dissolve((3000, BounceOut))",
    },
    DslShowcaseEntry {
        category: "Interpolation",
        title: "dissolve BounceIn",
        dsl: "fx::dissolve((3000, BounceIn))",
    },
    DslShowcaseEntry {
        category: "Interpolation",
        title: "dissolve ExpoOut",
        dsl: "fx::dissolve((3000, ExpoOut))",
    },
    DslShowcaseEntry {
        category: "Interpolation",
        title: "dissolve ElasticOut",
        dsl: "fx::dissolve((3000, ElasticOut))",
    },

    // ── Color Space ──────────────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Color Space",
        title: "fade_to_fg HSV",
        dsl: r#"
            fx::fade_to_fg(Color::Red, (3000, QuadOut))
                .with_color_space(ColorSpace::Hsv)
        "#,
    },
    DslShowcaseEntry {
        category: "Color Space",
        title: "fade_to_fg HSL",
        dsl: r#"
            fx::fade_to_fg(Color::Blue, (3000, SineOut))
                .with_color_space(ColorSpace::Hsl)
        "#,
    },
    DslShowcaseEntry {
        category: "Color Space",
        title: "fade_to_fg RGB",
        dsl: r#"
            fx::fade_to_fg(Color::Green, (3000, CubicOut))
                .with_color_space(ColorSpace::Rgb)
        "#,
    },

    // ── Evolution Effects ────────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Evolution",
        title: "evolve Shaded",
        dsl: "fx::evolve(EvolveSymbolSet::Shaded, (3000, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Evolution",
        title: "evolve Quadrants",
        dsl: "fx::evolve(EvolveSymbolSet::Quadrants, (3000, CubicOut))",
    },
    DslShowcaseEntry {
        category: "Evolution",
        title: "evolve BlocksHorizontal",
        dsl: "fx::evolve(EvolveSymbolSet::BlocksHorizontal, (3000, SineOut))",
    },
    DslShowcaseEntry {
        category: "Evolution",
        title: "evolve BlocksVertical",
        dsl: "fx::evolve(EvolveSymbolSet::BlocksVertical, (3000, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Evolution",
        title: "evolve_into Circles",
        dsl: r#"fx::evolve_into(EvolveSymbolSet::Circles, "◉", (3000, CubicOut))"#,
    },
    DslShowcaseEntry {
        category: "Evolution",
        title: "evolve_from Squares",
        dsl: r#"fx::evolve_from(EvolveSymbolSet::Squares, "■", (3000, QuadOut))"#,
    },

    // ── Advanced Compositions ────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Advanced",
        title: "parallel paint + dissolve",
        dsl: r#"
            fx::parallel(&[
                fx::paint_fg(Color::Rgb(255, 105, 180), (3000, QuadOut)),
                fx::dissolve((3000, CubicOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced",
        title: "sequence: evolve → dissolve",
        dsl: r#"
            fx::sequence(&[
                fx::evolve(EvolveSymbolSet::Shaded, (1500, QuadOut)),
                fx::dissolve((1500, CubicOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced",
        title: "parallel: sweep + fade + shift",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::parallel(&[
                fx::sweep_in(Motion::LeftToRight, 10, 3, bg, (3500, QuadOut)),
                fx::fade_to_fg(Color::Rgb(207, 181, 59), (3500, SineOut)),
                fx::hsl_shift_fg([20.0, 15.0, 10.0], (3500, CubicOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced",
        title: "dissolve radial + hsl",
        dsl: r#"
            fx::parallel(&[
                fx::dissolve((4000, QuadOut))
                    .with_pattern(RadialPattern::center()),
                fx::hsl_shift_fg([90.0, 30.0, 0.0], (4000, Linear))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced",
        title: "wave dissolve + fade gold",
        dsl: r#"
            fx::parallel(&[
                fx::dissolve((5000, Linear))
                    .with_pattern(WavePattern::new(
                        WaveLayer::new(Oscillator::sin(2.0, 1.0, 0.5))
                    )),
                fx::fade_to_fg(Color::Rgb(207, 181, 59), (5000, SineOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced",
        title: "spiral dissolve + paint",
        dsl: r#"
            fx::parallel(&[
                fx::dissolve((4000, Linear))
                    .with_pattern(SpiralPattern::center().with_arms(5)),
                fx::paint_fg(Color::Rgb(0, 200, 200), (4000, QuadOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced",
        title: "checkerboard + saturate",
        dsl: r#"
            fx::parallel(&[
                fx::coalesce((4000, QuadOut))
                    .with_pattern(CheckerboardPattern::default()),
                fx::saturate_fg(40.0, (4000, SineOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced",
        title: "diamond dissolve + darken",
        dsl: r#"
            fx::parallel(&[
                fx::dissolve((3500, CubicOut))
                    .with_pattern(DiamondPattern::center()),
                fx::darken_fg(30.0, (3500, QuadOut))
            ])
        "#,
    },
];

/// Number of procedurally generated effects beyond the static list.
/// Together with DSL_SHOWCASE these form an infinitely scrollable list.
const PROCEDURAL_EFFECT_COUNT: usize = 200;

/// Total showcase entries (static + procedural).
fn total_dsl_effects() -> usize {
    DSL_SHOWCASE.len() + PROCEDURAL_EFFECT_COUNT
}

/// Procedurally generate a DSL string and title for indices beyond the static
/// DSL_SHOWCASE table. Uses deterministic mixing of categories, colors,
/// interpolations, patterns, and timings so every index yields a unique
/// combination.
fn procedural_dsl_entry(index: usize) -> (String, String, String) {
    // Deterministic seed from index
    let seed = index.wrapping_mul(2654435761); // Knuth multiplicative hash

    // ── palettes ────────────────────────────────────────────────────────
    const COLORS: &[&str] = &[
        "Color::Rgb(207, 181, 59)",   // gold
        "Color::Rgb(184, 115, 51)",   // copper
        "Color::Rgb(192, 192, 192)",  // silver
        "Color::Rgb(0, 180, 180)",    // teal
        "Color::Rgb(255, 105, 180)",  // hot pink
        "Color::Rgb(100, 149, 237)",  // cornflower
        "Color::Rgb(255, 140, 0)",    // dark orange
        "Color::Rgb(138, 43, 226)",   // blue violet
        "Color::Rgb(50, 205, 50)",    // lime green
        "Color::Rgb(220, 20, 60)",    // crimson
        "Color::Red",
        "Color::Blue",
        "Color::Green",
        "Color::Cyan",
        "Color::Magenta",
        "Color::Yellow",
    ];

    const INTERPS: &[&str] = &[
        "Linear", "QuadOut", "QuadIn", "CubicOut", "CubicIn", "CubicInOut",
        "SineOut", "SineIn", "BounceOut", "ExpoOut", "ElasticOut", "QuadInOut",
    ];

    const MOTIONS: &[&str] = &[
        "Motion::LeftToRight",
        "Motion::RightToLeft",
        "Motion::UpToDown",
        "Motion::DownToUp",
    ];

    const HSL_SETS: &[(f32, f32, f32)] = &[
        (30.0, 20.0, 25.0),
        (-40.0, 15.0, -10.0),
        (60.0, 40.0, 30.0),
        (90.0, 50.0, 20.0),
        (20.0, -30.0, 40.0),
        (180.0, 0.0, 0.0),
        (45.0, -20.0, 15.0),
        (-90.0, 30.0, -20.0),
        (120.0, 10.0, 10.0),
        (15.0, 40.0, -15.0),
    ];

    const EVOLVE_SETS: &[&str] = &[
        "EvolveSymbolSet::Shaded",
        "EvolveSymbolSet::Quadrants",
        "EvolveSymbolSet::BlocksHorizontal",
        "EvolveSymbolSet::BlocksVertical",
        "EvolveSymbolSet::Circles",
        "EvolveSymbolSet::Squares",
    ];

    // Pick values from seed
    let pick = |arr_len: usize, salt: usize| -> usize {
        (seed.wrapping_add(salt).wrapping_mul(2246822519)) % arr_len
    };

    let color_a = COLORS[pick(COLORS.len(), 0)];
    let color_b = COLORS[pick(COLORS.len(), 7)];
    let interp = INTERPS[pick(INTERPS.len(), 1)];
    let interp2 = INTERPS[pick(INTERPS.len(), 11)];
    let motion = MOTIONS[pick(MOTIONS.len(), 2)];
    let hsl = HSL_SETS[pick(HSL_SETS.len(), 3)];
    let evolve = EVOLVE_SETS[pick(EVOLVE_SETS.len(), 4)];
    let duration = 2000 + (pick(5, 5) as u32) * 500;
    let duration2 = 1000 + (pick(4, 6) as u32) * 500;
    let arms = 2 + pick(6, 8) as u32;

    let category_idx = index % 12;

    let (category, title, dsl) = match category_idx {
        0 => {
            // Dissolve with pattern variation
            let patterns = [
                "RadialPattern::center()".to_string(),
                "DiamondPattern::center()".to_string(),
                format!("SpiralPattern::center().with_arms({})", arms),
                "DiagonalPattern::top_left_to_bottom_right()".to_string(),
                "CheckerboardPattern::default()".to_string(),
                "SweepPattern::left_to_right()".to_string(),
            ];
            let pat = &patterns[pick(patterns.len(), 9)];
            (
                "Procedural: Dissolve+Pattern".to_string(),
                format!("dissolve #{} pattern", index + 1),
                format!(
                    "fx::dissolve(({}, {}))\n    .with_pattern({})",
                    duration, interp, pat
                ),
            )
        }
        1 => {
            // Coalesce with pattern
            let patterns = [
                "RadialPattern::center()".to_string(),
                "DiamondPattern::center()".to_string(),
                format!("SpiralPattern::center().with_arms({})", arms),
                "InvertedPattern::new(RadialPattern::center())".to_string(),
            ];
            let pat = &patterns[pick(patterns.len(), 10)];
            (
                "Procedural: Coalesce+Pattern".to_string(),
                format!("coalesce #{} pattern", index + 1),
                format!(
                    "fx::coalesce(({}, {}))\n    .with_pattern({})",
                    duration, interp, pat
                ),
            )
        }
        2 => {
            // Sweep with varying params
            let cells = 6 + pick(15, 12) as u16;
            let gap = pick(5, 13) as u16;
            (
                "Procedural: Sweep".to_string(),
                format!("sweep #{} {}", index + 1, &motion[8..]),
                format!(
                    "let bg = Color::Rgb(8, 9, 14);\nfx::sweep_in({}, {}, {}, bg, ({}, {}))",
                    motion, cells, gap, duration, interp
                ),
            )
        }
        3 => {
            // Slide with varying params
            let cells = 4 + pick(10, 14) as u16;
            let gap = 1 + pick(4, 15) as u16;
            (
                "Procedural: Slide".to_string(),
                format!("slide #{} {}", index + 1, &motion[8..]),
                format!(
                    "let bg = Color::Rgb(8, 9, 14);\nfx::slide_in({}, {}, {}, bg, ({}, {}))",
                    motion, cells, gap, duration, interp
                ),
            )
        }
        4 => {
            // Fade to fg color
            (
                "Procedural: Fade".to_string(),
                format!("fade_to_fg #{}", index + 1),
                format!(
                    "fx::fade_to_fg({}, ({}, {}))",
                    color_a, duration, interp
                ),
            )
        }
        5 => {
            // HSL shift
            (
                "Procedural: HSL Shift".to_string(),
                format!("hsl_shift #{}", index + 1),
                format!(
                    "fx::hsl_shift_fg([{:.1}, {:.1}, {:.1}], ({}, {}))",
                    hsl.0, hsl.1, hsl.2, duration, interp
                ),
            )
        }
        6 => {
            // Paint fg
            (
                "Procedural: Paint".to_string(),
                format!("paint_fg #{}", index + 1),
                format!(
                    "fx::paint_fg({}, ({}, {}))",
                    color_a, duration, interp
                ),
            )
        }
        7 => {
            // Evolve
            (
                "Procedural: Evolution".to_string(),
                format!("evolve #{}", index + 1),
                format!(
                    "fx::evolve({}, ({}, {}))",
                    evolve, duration, interp
                ),
            )
        }
        8 => {
            // Sequence: dissolve → coalesce with different interps
            (
                "Procedural: Sequence".to_string(),
                format!("seq dissolve→coalesce #{}", index + 1),
                format!(
                    "fx::sequence(&[\n    fx::dissolve(({}, {})),\n    fx::coalesce(({}, {}))\n])",
                    duration2, interp, duration2, interp2
                ),
            )
        }
        9 => {
            // Parallel: fade + dissolve with pattern
            let patterns = [
                "RadialPattern::center()".to_string(),
                "DiamondPattern::center()".to_string(),
                format!("SpiralPattern::center().with_arms({})", arms),
            ];
            let pat = &patterns[pick(patterns.len(), 16)];
            (
                "Procedural: Parallel".to_string(),
                format!("parallel fade+dissolve #{}", index + 1),
                format!(
                    "fx::parallel(&[\n    fx::fade_to_fg({}, ({}, {})),\n    fx::dissolve(({}, {}))\n        .with_pattern({})\n])",
                    color_a, duration, interp, duration, interp2, pat
                ),
            )
        }
        10 => {
            // Wave pattern dissolve
            let kx = 1.0 + (pick(5, 17) as f32) * 0.5;
            let ky = (pick(4, 18) as f32) * 1.0;
            let kt = 0.3 + (pick(4, 19) as f32) * 0.3;
            let wave_types = ["sin", "cos", "triangle", "sawtooth"];
            let wt = wave_types[pick(wave_types.len(), 20)];
            (
                "Procedural: Wave".to_string(),
                format!("wave {} dissolve #{}", wt, index + 1),
                format!(
                    "fx::dissolve(({}, Linear))\n    .with_pattern(WavePattern::new(\n        WaveLayer::new(Oscillator::{}({:.1}, {:.1}, {:.1}))\n    ))",
                    duration + 1000, wt, kx, ky, kt
                ),
            )
        }
        11 => {
            // Sequence: paint chain
            (
                "Procedural: Paint Chain".to_string(),
                format!("paint chain #{}", index + 1),
                format!(
                    "fx::sequence(&[\n    fx::paint_fg({}, {}),\n    fx::paint_fg({}, {})\n])",
                    color_a, duration2, color_b, duration2
                ),
            )
        }
        _ => unreachable!(),
    };

    (category, title, dsl)
}

/// Compile a DSL expression string into a looping (repeating + ping_pong) Effect.
fn compile_dsl_effect(dsl_src: &str) -> Option<Effect> {
    let dsl = EffectDsl::new();
    // Wrap the user expression in repeating(ping_pong(...)) for infinite loop
    let wrapped = format!("fx::repeating(fx::ping_pong({}))", dsl_src.trim());
    match dsl.compiler().compile(&wrapped) {
        Ok(effect) => Some(effect),
        Err(_) => {
            // Fallback: try without wrapping (some effects might not support ping_pong)
            match dsl.compiler().compile(dsl_src.trim()) {
                Ok(effect) => Some(fx::repeating(fx::ping_pong(effect))),
                Err(_) => None,
            }
        }
    }
}

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
    (
        "Unified Layout Design",
        "2025-05-15",
        "Traditional responsive design uses breakpoints to switch between \
         mobile and desktop layouts. This site takes a different approach: \
         there is only one layout that works everywhere. The terminal grid \
         scales naturally to any screen size, and touch gestures work \
         alongside mouse and keyboard input. No media queries, no \
         breakpoints, no separate code paths. The same Rust code renders \
         identically on a phone, tablet, or ultrawide monitor.",
    ),
    (
        "WebAssembly Performance",
        "2025-06-01",
        "Compiling Rust to WebAssembly gives near-native performance in \
         the browser. Grift's arena allocator avoids garbage collection \
         pauses entirely — memory is managed through a mark-and-sweep \
         collector that runs on the fixed-size arena. Combined with \
         Ratzilla's WebGL2 renderer, the UI maintains smooth 60fps \
         animation even on mid-range mobile devices.",
    ),
    (
        "TachyonFX: Shader Effects for TUIs",
        "2025-07-10",
        "TachyonFX brings shader-like visual effects to terminal UIs. \
         Effects like fade, sweep, slide, coalesce, and HSL shift can \
         be composed with combinators like ping_pong and repeating. \
         Each effect operates on a rectangular cell region and tracks \
         its own timing via EffectTimer. The library integrates with \
         Ratatui's rendering pipeline through the EffectRenderer trait, \
         making it easy to add polish to any terminal application.",
    ),
    (
        "Arena Allocation in Grift",
        "2025-08-05",
        "Grift uses a fixed-size arena for all allocations. The arena \
         is a contiguous array of cells, each holding a Lisp value. \
         Const generics set the capacity at compile time — no runtime \
         overhead, no dynamic allocation, no unsafe code. A mark-and-sweep \
         garbage collector reclaims unreachable cells. This design makes \
         Grift suitable for embedded systems with no heap and for WASM \
         targets where memory management must be predictable.",
    ),
];

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Home,
    Repl,
    Docs,
    Blog,
    About,
    Effects,
    Clock,
    Matrix,
}

impl Page {
    const ALL: [Page; 8] = [Page::Home, Page::Repl, Page::Docs, Page::Blog, Page::About, Page::Effects, Page::Clock, Page::Matrix];

    fn title(self) -> &'static str {
        match self {
            Page::Home => "Home",
            Page::Repl => "REPL",
            Page::Docs => "Docs",
            Page::Blog => "Blog",
            Page::About => "About",
            Page::Effects => "Effects",
            Page::Clock => "Clock",
            Page::Matrix => "Matrix",
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|&p| p == self).unwrap_or(0)
    }
}

#[derive(Clone, Copy)]
enum ScrollTarget {
    Home,
    About,
    Docs,
}

struct MatrixColumn {
    head: u16,    // current head row position
    length: u16,  // trail length
    speed: u16,   // ticks per step
    counter: u16, // tick counter
    chars: Vec<char>, // characters in this column
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
    about_scroll: usize,
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
    link_hover_effects: Vec<(usize, Effect)>,
    last_hovered_link: Option<usize>,
    // DSL effects showcase state (infinitely scrollable)
    dsl_effects_scroll: usize,
    dsl_effects_cache: Vec<Option<Effect>>,
    frame_elapsed: Duration,
    // Matrix rain state
    matrix_columns: Vec<MatrixColumn>,
    matrix_initialized: bool,
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
            about_scroll: 0,
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
            link_hover_effects: Vec::new(),
            last_hovered_link: None,
            dsl_effects_scroll: 0,
            dsl_effects_cache: Vec::new(),
            frame_elapsed: Duration::from_millis(0),
            matrix_columns: Vec::new(),
            matrix_initialized: false,
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
            Page::About => fx::sweep_in(
                Motion::UpToDown,
                8,
                2,
                dark,
                EffectTimer::from_ms(500, Interpolation::QuadOut),
            ),
            Page::Effects => fx::coalesce(EffectTimer::from_ms(500, Interpolation::SineOut)),
            Page::Clock => fx::fade_from(
                dark,
                dark,
                EffectTimer::from_ms(400, Interpolation::CubicOut),
            ),
            Page::Matrix => fx::sweep_in(
                Motion::UpToDown,
                12,
                2,
                dark,
                EffectTimer::from_ms(600, Interpolation::QuadOut),
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
            // Clear stale click areas from previous page to prevent phantom clicks on mobile
            self.link_areas.clear();
            self.blog_item_areas.clear();
            self.blog_back_area = Rect::default();
            self.scroll_up_area = Rect::default();
            self.scroll_down_area = Rect::default();
            self.blog_list_area = Rect::default();
            self.blog_content_area = Rect::default();
            self.trigger_transition();
            // Focus/blur hidden input for REPL virtual keyboard
            if page == Page::Repl {
                let _ = web_sys::js_sys::eval("window._replTabActive=true;window._focusReplInput&&window._focusReplInput()");
            } else {
                let _ = web_sys::js_sys::eval("window._replTabActive=false;window._blurReplInput&&window._blurReplInput()");
            }
        }
    }

    fn switch_to_prev_tab(&mut self) {
        let idx = self.page.index();
        if idx > 0 {
            self.switch_page(Page::ALL[idx - 1]);
        }
    }

    fn switch_to_next_tab(&mut self) {
        let idx = self.page.index();
        if idx + 1 < Page::ALL.len() {
            self.switch_page(Page::ALL[idx + 1]);
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
            Page::About => self.handle_scroll_event(key, ScrollTarget::About),
            Page::Effects => self.handle_effects_event(key),
            Page::Clock | Page::Matrix => self.handle_demo_tab_event(key),
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

            // Check link clicks on About page
            if self.page == Page::About {
                for (i, area) in self.link_areas.iter().enumerate() {
                    if col >= area.x
                        && col < area.right()
                        && row >= area.y
                        && row < area.bottom()
                    {
                        if let Some((_, url, _)) = LINKS.get(i) {
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
                // Skip modifier-held keys (Ctrl+V paste is handled by JS paste event)
                if key.ctrl || key.alt {
                    return;
                }
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
            // Viewing the list: arrow keys scroll only, tap/click selects
            match key.code {
                KeyCode::Up => {
                    self.blog_scroll = self.blog_scroll.saturating_sub(1);
                }
                KeyCode::Down => {
                    self.blog_scroll += 1;
                }
                _ => {}
            }
        }
    }

    fn handle_scroll_event(&mut self, key: KeyEvent, target: ScrollTarget) {
        let scroll = match target {
            ScrollTarget::Home => &mut self.home_scroll,
            ScrollTarget::About => &mut self.about_scroll,
            ScrollTarget::Docs => &mut self.doc_scroll,
        };
        let step = 2;
        match key.code {
            KeyCode::Up => {
                *scroll = scroll.saturating_sub(step);
            }
            KeyCode::Down => {
                *scroll += step;
            }
            KeyCode::Left => {
                self.switch_to_prev_tab();
            }
            KeyCode::Right => {
                self.switch_to_next_tab();
            }
            _ => {}
        }
    }

    fn handle_effects_event(&mut self, key: KeyEvent) {
        let step = 2;
        match key.code {
            KeyCode::Up => {
                self.dsl_effects_scroll = self.dsl_effects_scroll.saturating_sub(step);
            }
            KeyCode::Down => {
                self.dsl_effects_scroll += step;
            }
            KeyCode::Left => {
                // Jump back by one full page of entries
                self.dsl_effects_scroll = self.dsl_effects_scroll.saturating_sub(20);
            }
            KeyCode::Right => {
                // Jump forward by one full page of entries
                self.dsl_effects_scroll += 20;
            }
            _ => {}
        }
    }

    fn handle_demo_tab_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Left => {
                self.switch_to_prev_tab();
            }
            KeyCode::Right => {
                self.switch_to_next_tab();
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
        self.frame_elapsed = elapsed;

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
        let h_margin = (full_area.width / MARGIN_DIVISOR).min(2);
        let v_margin = (full_area.height / MARGIN_DIVISOR).min(1);

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
                // Subtle warm copper/gold hsl shift
                let fg_shift = [8.0, 10.0, 6.0];
                let timer = (1800, Interpolation::SineIn);
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
                    let sweep = fx::sweep_in(
                        Motion::LeftToRight,
                        6,
                        2,
                        Color::Rgb(8, 9, 14),
                        EffectTimer::from_ms(350, Interpolation::QuadOut),
                    );
                    self.tab_hover_effects.push((idx, sweep));
                    if let Some(tab_rect) = self.tab_rects.get(idx).copied() {
                        let shift = fx::hsl_shift_fg(
                            [20.0, 10.0, 14.0],
                            (450, Interpolation::SineOut),
                        );
                        self.btn_effects.push((tab_rect, shift));
                    }
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
            Page::About => self.render_about(frame, content_area),
            Page::Effects => self.render_effects(frame, content_area),
            Page::Clock => self.render_clock(frame, content_area),
            Page::Matrix => self.render_matrix(frame, content_area),
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
        if self.page == Page::About {
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
        let divider_width: u16 = 1;
        let pad: u16 = 1; // space on each side of the title
        let inner_x = area.x + 1; // after left border
        let tab_row = area.y + 1;

        // First pass: compute tab positions relative to line start
        let mut tab_offsets: Vec<(u16, u16)> = Vec::new(); // (offset_from_line_start, width)
        let mut line_pos: u16 = 0;
        for (i, p) in Page::ALL.iter().enumerate() {
            if i > 0 {
                line_pos += divider_width;
            }
            let padded_len = p.title().len() as u16 + pad * 2;
            tab_offsets.push((line_pos, padded_len));
            line_pos += padded_len;
        }
        let total_line_width = line_pos;

        // Clamp horizontal scroll
        let inner_width = area.width.saturating_sub(2);
        let max_h_scroll = total_line_width.saturating_sub(inner_width) as usize;
        self.tab_h_scroll = self.tab_h_scroll.min(max_h_scroll);

        // Compute center offset (matching Paragraph's Alignment::Center behavior)
        let center_offset = if inner_width > total_line_width {
            (inner_width - total_line_width) / 2
        } else {
            0
        };

        // Build final tab_rects with center offset and scroll applied, clipped to visible area
        let visible_left = inner_x;
        let visible_right = inner_x.saturating_add(inner_width);
        self.tab_rects.clear();
        for (offset, width) in &tab_offsets {
            let raw_x = (inner_x as i32)
                + (center_offset as i32)
                + (*offset as i32)
                - (self.tab_h_scroll as i32);
            let raw_right = raw_x + (*width as i32);
            let clipped_x = (raw_x.max(visible_left as i32) as u16).min(visible_right);
            let clipped_right = (raw_right.max(visible_left as i32) as u16).min(visible_right);
            if clipped_right > clipped_x {
                self.tab_rects.push(Rect::new(clipped_x, tab_row, clipped_right - clipped_x, 1));
            } else {
                // Tab is fully scrolled out of view — push empty rect to preserve indexing
                self.tab_rects.push(Rect::default());
            }
        }

        let mut spans: Vec<Span> = Vec::new();
        for (i, p) in Page::ALL.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled("│", Style::default().fg(Color::Rgb(100, 105, 115))));
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
            let padded_title = format!(" {} ", p.title());
            spans.push(Span::styled(padded_title, style));
        }

        let tab_line = Line::from(spans);
        let tab_paragraph = Paragraph::new(tab_line)
            .alignment(Alignment::Center)
            .scroll((0, self.tab_h_scroll as u16))
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Color::Rgb(55, 60, 70))
                    .title(Line::from(" GRIFT.RS ").alignment(Alignment::Center))
                    .title_style(Style::default().fg(Color::Rgb(207, 181, 59)).bold()),
            );

        frame.render_widget(tab_paragraph, area);

        // Render horizontal scrollbar on the tab bar bottom border if tabs overflow
        self.render_horizontal_scrollbar(frame, area, self.tab_h_scroll, max_h_scroll);
    }

    fn render_home(&mut self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        // Grift section
        lines.push(Line::styled("── Grift ──", Style::default().fg(Color::Rgb(207, 181, 59)).bold()));
        lines.push(Line::from(""));
        for l in DESCRIPTION.lines() {
            lines.push(Line::styled(l, Style::default().fg(Color::Rgb(170, 175, 185))));
        }
        lines.push(Line::from(""));

        // Vau Calculus / Fexprs section
        lines.push(Line::styled("── Fexprs & Vau Calculus ──", Style::default().fg(Color::Rgb(184, 115, 51)).bold()));
        lines.push(Line::from(""));
        for l in VAU_INFO.lines() {
            lines.push(Line::styled(l, Style::default().fg(Color::Rgb(170, 175, 185))));
        }
        lines.push(Line::from(""));

        // First-Class Everything section
        lines.push(Line::styled("── First-Class Everything ──", Style::default().fg(Color::Rgb(200, 200, 210)).bold()));
        lines.push(Line::from(""));
        for l in FIRST_CLASS_INFO.lines() {
            lines.push(Line::styled(l, Style::default().fg(Color::Rgb(170, 175, 185))));
        }
        lines.push(Line::from(""));

        // Implementation section
        lines.push(Line::styled("── Implementation ──", Style::default().fg(Color::Rgb(222, 165, 132)).bold()));
        lines.push(Line::from(""));
        for l in IMPL_INFO.lines() {
            lines.push(Line::styled(l, Style::default().fg(Color::Rgb(170, 175, 185))));
        }

        lines.push(Line::from(""));

        // Why Grift section
        lines.push(Line::styled("── Why Grift? ──", Style::default().fg(Color::Rgb(160, 175, 195)).bold()));
        lines.push(Line::from(""));
        let why_grift = "Most Lisps distinguish between functions and macros at a fundamental level. Grift eliminates this distinction entirely through vau calculus. Every combiner is an operative that can choose whether to evaluate its arguments. This makes the language simpler, more uniform, and more powerful. If you can write a function, you can write a macro — they are the same thing.";
        lines.push(Line::styled(why_grift, Style::default().fg(Color::Rgb(170, 175, 185))));
        lines.push(Line::from(""));

        // This Site section
        lines.push(Line::styled("── This Site ──", Style::default().fg(Color::Rgb(207, 181, 59)).bold()));
        lines.push(Line::from(""));
        let this_site = "Everything you see is a Rust terminal UI compiled to WebAssembly and rendered to an HTML canvas via Ratzilla. TachyonFX provides the animated background, page transitions, and hover effects. There is no HTML layout, no CSS styling, and no JavaScript framework — just a Rust application drawing characters to a terminal grid. The same layout works on every device and screen size.";
        lines.push(Line::styled(this_site, Style::default().fg(Color::Rgb(170, 175, 185))));
        lines.push(Line::from(""));

        // Getting Started section
        lines.push(Line::styled("── Getting Started ──", Style::default().fg(Color::Rgb(184, 115, 51)).bold()));
        lines.push(Line::from(""));
        let getting_started = "Try Grift right now — switch to the REPL tab and type (+ 1 2). Browse the Docs tab for the full language reference. Check the Effects tab to see TachyonFX visual effects in action. All tabs are accessible via touch, mouse, or keyboard.";
        lines.push(Line::styled(getting_started, Style::default().fg(Color::Rgb(170, 175, 185))));
        lines.push(Line::from(""));

        // Features at a Glance section
        lines.push(Line::styled("── Features at a Glance ──", Style::default().fg(Color::Rgb(160, 175, 195)).bold()));
        lines.push(Line::from(""));
        let features = "• Interactive REPL with full Grift interpreter\n• Animated background and page transitions\n• Clickable tabs, links, and buttons\n• Mobile-first touch gesture support\n• Momentum scrolling and swipe navigation\n• Zero JavaScript frameworks — pure Rust + WASM";
        for l in features.lines() {
            lines.push(Line::styled(l, Style::default().fg(Color::Rgb(170, 175, 185))));
        }

        let mut scroll = self.home_scroll;
        self.render_scrollable_content(
            frame, area, lines, &mut scroll,
            None,
            None,
            "swipe ↕ ↔",
        );
        self.home_scroll = scroll;
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

        let visible_height = history_area.height.saturating_sub(2) as usize;
        let content_width = history_area.width.saturating_sub(2) as usize;
        let total_wrapped = Self::wrapped_line_count(&history_lines, content_width);
        let max_scroll = total_wrapped.saturating_sub(visible_height);
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

        // Render vertical scrollbar on REPL output if it overflows
        self.render_vertical_scrollbar(frame, history_area, scroll, max_scroll);
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
        let all_docs = [DOC_BASICS, DOC_FORMS, DOC_ADVANCED, DOC_ENVIRONMENTS, DOC_ERRORS];
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

                let visible_height = scroll_area.height.saturating_sub(2) as usize;
                let content_width = scroll_area.width.saturating_sub(2) as usize;
                let total_wrapped = Self::wrapped_line_count(&lines, content_width);
                let max_scroll = total_wrapped.saturating_sub(visible_height);
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

                // Render vertical scrollbar on blog post content
                self.render_vertical_scrollbar(frame, scroll_area, self.blog_scroll, max_scroll);

                self.render_scroll_arrows(frame, nav_bar, self.blog_scroll, max_scroll, "swipe ↕");
            }
            return;
        }

        // Show scrollable list of blog titles as clickable buttons
        self.blog_list_area = area;
        self.blog_content_area = Rect::default();

        let mut lines: Vec<Line> = Vec::new();
        let mut blog_line_indices: Vec<usize> = Vec::new(); // line index for each blog entry

        for (i, (title, date, _)) in BLOG_ENTRIES.iter().enumerate() {
            blog_line_indices.push(lines.len());
            let hovered = self
                .blog_item_areas
                .get(i)
                .is_some_and(|r| self.is_hovered(*r));
            let style = if hovered {
                Style::default().fg(Color::Rgb(255, 255, 255)).bold().add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(Color::Rgb(200, 200, 210)).bold()
            };
            let marker = if hovered { "▸ " } else { "  " };
            lines.push(Line::from(format!("{marker}{title}")).style(style));
            lines.push(Line::styled(format!("    {date}"), Style::default().fg(Color::Rgb(75, 80, 90))));
            lines.push(Line::from(""));
        }

        let mut scroll = self.blog_scroll;
        let scroll_area = self.render_scrollable_content(
            frame, area, lines, &mut scroll,
            Some(" Blog ".bold().fg(Color::Rgb(200, 200, 210)).into()),
            Some(" Posts — tap to read ".bold().fg(Color::Rgb(184, 115, 51)).into()),
            "swipe ↕ │ tap a post",
        );
        self.blog_scroll = scroll;

        // Compute click areas for blog titles relative to the scroll position
        let content_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(40, 44, 52));
        let content_inner = content_block.inner(scroll_area);

        self.blog_item_areas.clear();
        for &line_idx in blog_line_indices.iter() {
            if line_idx >= self.blog_scroll {
                let visible_row = (line_idx - self.blog_scroll) as u16;
                if visible_row < content_inner.height {
                    // Click area covers the title line (1 row)
                    self.blog_item_areas.push(Rect::new(
                        content_inner.x,
                        content_inner.y + visible_row,
                        content_inner.width,
                        1,
                    ));
                } else {
                    self.blog_item_areas.push(Rect::default());
                }
            } else {
                self.blog_item_areas.push(Rect::default());
            }
        }
    }

    fn render_about(&mut self, frame: &mut Frame, area: Rect) {
        // Build the About page: bio → grift description → links with descriptions → interesting info
        let mut lines: Vec<Line> = Vec::new();

        // ── Bio ──
        lines.push(Line::styled("── gold silver copper ──", Style::default().fg(Color::Rgb(207, 181, 59)).bold()));
        lines.push(Line::from(""));
        lines.push(Line::styled(
            "Software developer building open-source tools in Rust. Interested in programming language design, terminal user interfaces, WebAssembly, and making the web a stranger and more interesting place. Creator of Grift and this terminal-in-a-browser website.",
            Style::default().fg(Color::Rgb(170, 175, 185)),
        ));
        lines.push(Line::from(""));

        // ── Grift ──
        lines.push(Line::styled("── Grift ──", Style::default().fg(Color::Rgb(184, 115, 51)).bold()));
        lines.push(Line::from(""));
        for l in DESCRIPTION.lines() {
            lines.push(Line::styled(l, Style::default().fg(Color::Rgb(170, 175, 185))));
        }
        lines.push(Line::from(""));

        // ── Links ──
        lines.push(Line::styled("── Links ──", Style::default().fg(Color::Rgb(200, 200, 210)).bold()));
        lines.push(Line::styled("────────────", Style::default().fg(Color::Rgb(140, 145, 155))));

        // Link lines: each link label is on its own line, followed by a description line
        // We track the line index where each link label appears for click tracking
        let links_start_line = lines.len();
        for (label, _url, desc) in LINKS.iter() {
            lines.push(Line::styled(format!("  {label}"), Style::default().fg(Color::Rgb(160, 175, 195))));
            lines.push(Line::styled(format!("    — {desc}"), Style::default().fg(Color::Rgb(110, 115, 125))));
        }
        let _links_end_line = lines.len();

        lines.push(Line::from(""));

        // ── Interesting info (merged from showcase) ──
        lines.push(Line::styled("── This Website ──", Style::default().fg(Color::Rgb(207, 181, 59)).bold()));
        lines.push(Line::from(""));
        lines.push(Line::styled(
            "Everything you see is a Rust terminal UI compiled to WebAssembly and rendered to an HTML canvas via Ratzilla. TachyonFX provides the animated background, page transitions, and hover effects. There is no HTML layout, no CSS styling, and no JavaScript framework — just a Rust application drawing characters to a terminal grid.",
            Style::default().fg(Color::Rgb(170, 175, 185)),
        ));
        lines.push(Line::from(""));

        lines.push(Line::styled("── How It Works ──", Style::default().fg(Color::Rgb(184, 115, 51)).bold()));
        lines.push(Line::from(""));
        lines.push(Line::styled(
            "Traditional web apps use HTML/CSS/JavaScript to render DOM elements. This site takes a different approach: the entire UI is a Rust application compiled to WASM, rendering a terminal grid to an HTML canvas. There is no DOM manipulation, no CSS layout engine, and no JavaScript framework involved.",
            Style::default().fg(Color::Rgb(170, 175, 185)),
        ));
        lines.push(Line::from(""));

        lines.push(Line::styled("── Built With ──", Style::default().fg(Color::Rgb(200, 200, 210)).bold()));
        lines.push(Line::from(""));
        lines.push(Line::styled("  • Ratzilla — terminal web apps with Rust + WASM", Style::default().fg(Color::Rgb(170, 175, 185))));
        lines.push(Line::styled("  • Ratatui — terminal UI framework for Rust", Style::default().fg(Color::Rgb(170, 175, 185))));
        lines.push(Line::styled("  • TachyonFX — shader-like effects for terminal UIs", Style::default().fg(Color::Rgb(170, 175, 185))));
        lines.push(Line::styled("  • Grift — minimalistic Lisp with vau calculus", Style::default().fg(Color::Rgb(170, 175, 185))));
        lines.push(Line::styled("  • WebGL2 rendering at 60fps on modern devices", Style::default().fg(Color::Rgb(170, 175, 185))));
        lines.push(Line::from(""));

        lines.push(Line::styled("── Interactions ──", Style::default().fg(Color::Rgb(207, 181, 59)).bold()));
        lines.push(Line::from(""));
        lines.push(Line::styled("  • Swipe LEFT / RIGHT to switch between tabs", Style::default().fg(Color::Rgb(170, 175, 185))));
        lines.push(Line::styled("  • Swipe UP / DOWN to scroll content", Style::default().fg(Color::Rgb(170, 175, 185))));
        lines.push(Line::styled("  • Tap on tabs, links, and buttons to interact", Style::default().fg(Color::Rgb(170, 175, 185))));
        lines.push(Line::styled("  • Mouse wheel scrolling works everywhere", Style::default().fg(Color::Rgb(170, 175, 185))));
        lines.push(Line::styled("  • Keyboard input works on the REPL tab", Style::default().fg(Color::Rgb(170, 175, 185))));
        lines.push(Line::styled("  • Paste text with Ctrl+V / Cmd+V in the REPL", Style::default().fg(Color::Rgb(170, 175, 185))));
        lines.push(Line::from(""));

        lines.push(Line::styled("── Performance ──", Style::default().fg(Color::Rgb(184, 115, 51)).bold()));
        lines.push(Line::from(""));
        lines.push(Line::styled("  • WASM binary is ~200KB compressed", Style::default().fg(Color::Rgb(170, 175, 185))));
        lines.push(Line::styled("  • No garbage collection pauses (arena allocator)", Style::default().fg(Color::Rgb(170, 175, 185))));
        lines.push(Line::styled("  • Minimal memory footprint — fixed arena with const generics", Style::default().fg(Color::Rgb(170, 175, 185))));
        lines.push(Line::styled("  • Single codebase for all screen sizes", Style::default().fg(Color::Rgb(170, 175, 185))));
        lines.push(Line::styled("  • Full offline capability once cached by the browser", Style::default().fg(Color::Rgb(170, 175, 185))));

        // Use render_scrollable_content which handles the bordered block, scroll, and nav bar
        let mut scroll = self.about_scroll;
        self.render_scrollable_content(
            frame, area, lines, &mut scroll,
            Some(" About ".bold().fg(Color::Rgb(207, 181, 59)).into()),
            Some(" gold.silver.copper ".bold().fg(Color::Rgb(184, 115, 51)).into()),
            "swipe ↕ │ tap links",
        );
        self.about_scroll = scroll;

        // Track clickable link areas inside the scrollable view
        // The scrollable content is rendered inside a double-bordered area
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(55, 60, 70));
        let inner = block.inner(area);
        let [scroll_area, _nav_bar] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(inner);
        let content_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(40, 44, 52));
        let content_inner = content_block.inner(scroll_area);

        self.link_areas.clear();
        for i in 0..LINKS.len() {
            // Each link takes 2 lines (label + description), starting at links_start_line
            let line_idx = links_start_line + i * 2;
            if line_idx >= self.about_scroll {
                let visible_row = (line_idx - self.about_scroll) as u16;
                if visible_row < content_inner.height {
                    let link_area = Rect::new(content_inner.x, content_inner.y + visible_row, content_inner.width, 1);
                    self.link_areas.push(link_area);
                }
            }
        }

        // Apply hover styling to link lines (re-render hovered links with REVERSED style)
        for (i, link_area) in self.link_areas.iter().enumerate() {
            if self.is_hovered(*link_area) {
                if let Some((label, _, _)) = LINKS.get(i) {
                    let text = format!("  {label}");
                    let style = Style::default().fg(Color::Rgb(160, 175, 195)).add_modifier(Modifier::REVERSED);
                    let buf = frame.buffer_mut();
                    for (k, ch) in text.chars().enumerate() {
                        let x = link_area.x + k as u16;
                        if x < link_area.right() {
                            if let Some(cell) = buf.cell_mut(Position::new(x, link_area.y)) {
                                cell.set_char(ch);
                                cell.set_style(style);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Compute total display rows after wrapping lines to the given width.
    ///
    /// Simulates ratatui's `WordWrapper` with `trim: false` to produce an
    /// accurate count that matches what `Paragraph::wrap(Wrap { trim: false })`
    /// will actually render.
    fn wrapped_line_count(lines: &[Line], wrap_width: usize) -> usize {
        if wrap_width == 0 {
            return lines.len();
        }
        let max_w = wrap_width;

        lines.iter().map(|line| {
            let w = line.width();
            if w <= max_w {
                return 1;
            }

            // Collect text from all spans for word-wrap simulation
            let text: String = line.iter().map(|span| span.content.as_ref()).collect();

            let mut count: usize = 0;
            let mut line_w: usize = 0;
            let mut word_w: usize = 0;
            let mut ws_w: usize = 0;
            let mut pending_empty = true;
            let mut non_ws_prev = false;

            for ch in text.chars() {
                let is_ws = ch.is_whitespace();
                let sym_w = UnicodeWidthChar::width(ch).unwrap_or(0);

                // Skip characters wider than the line (matching ratatui's WordWrapper)
                if sym_w > max_w { continue; }

                let word_found = non_ws_prev && is_ws;
                let untrimmed_overflow = pending_empty
                    && (word_w + ws_w + sym_w > max_w);

                if word_found || untrimmed_overflow {
                    line_w += ws_w + word_w;
                    pending_empty = line_w == 0;
                    ws_w = 0;
                    word_w = 0;
                }

                let line_full = line_w >= max_w;
                let pending_overflow = sym_w > 0
                    && (line_w + ws_w + word_w >= max_w);

                if line_full || pending_overflow {
                    count += 1;
                    pending_empty = true;
                    line_w = 0;
                    ws_w = 0;

                    if is_ws {
                        non_ws_prev = false;
                        continue;
                    }
                }

                if is_ws {
                    ws_w += sym_w;
                } else {
                    word_w += sym_w;
                }

                non_ws_prev = !is_ws;
            }

            count + 1
        }).sum()
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

        let visible_height = scroll_area.height.saturating_sub(2) as usize;
        let content_width = scroll_area.width.saturating_sub(2) as usize;
        let total_wrapped = Self::wrapped_line_count(&lines, content_width);
        let max_scroll = total_wrapped.saturating_sub(visible_height);
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

        // Render vertical scrollbar on the right border if content overflows
        self.render_vertical_scrollbar(frame, scroll_area, *scroll, max_scroll);

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

    /// Render a vertical scrollbar on the right border of the given area.
    /// Only renders if the content overflows (max_scroll > 0).
    fn render_vertical_scrollbar(&self, frame: &mut Frame, area: Rect, scroll: usize, max_scroll: usize) {
        if max_scroll == 0 || area.height < 4 {
            return; // Content fits — no scrollbar needed
        }
        let track_height = area.height.saturating_sub(2) as usize; // exclude top/bottom border
        if track_height == 0 {
            return;
        }
        // Compute thumb size and position
        let thumb_size = (track_height * track_height / (track_height + max_scroll)).max(1);
        let thumb_pos = if max_scroll > 0 {
            scroll * (track_height - thumb_size) / max_scroll
        } else {
            0
        };
        let bar_x = area.right().saturating_sub(1); // right border column
        let buf = frame.buffer_mut();
        for i in 0..track_height {
            let y = area.y + 1 + i as u16; // skip top border
            let pos = Position::new(bar_x, y);
            if let Some(cell) = buf.cell_mut(pos) {
                if i >= thumb_pos && i < thumb_pos + thumb_size {
                    // Thumb: bright scrollbar indicator
                    cell.set_char('┃');
                    cell.set_fg(Color::Rgb(140, 145, 160));
                } else {
                    // Track: subtle indicator
                    cell.set_char('│');
                    cell.set_fg(Color::Rgb(35, 38, 46));
                }
            }
        }
    }

    /// Render a horizontal scrollbar on the bottom border of the given area.
    /// Only renders if the content overflows (max_h_scroll > 0).
    fn render_horizontal_scrollbar(&self, frame: &mut Frame, area: Rect, scroll: usize, max_scroll: usize) {
        if max_scroll == 0 || area.width < 4 {
            return; // Content fits — no scrollbar needed
        }
        let track_width = area.width.saturating_sub(2) as usize; // exclude left/right border
        if track_width == 0 {
            return;
        }
        let thumb_size = (track_width * track_width / (track_width + max_scroll)).max(1);
        let thumb_pos = if max_scroll > 0 {
            scroll * (track_width - thumb_size) / max_scroll
        } else {
            0
        };
        let bar_y = area.bottom().saturating_sub(1); // bottom border row
        let buf = frame.buffer_mut();
        for i in 0..track_width {
            let x = area.x + 1 + i as u16; // skip left border
            let pos = Position::new(x, bar_y);
            if let Some(cell) = buf.cell_mut(pos) {
                if i >= thumb_pos && i < thumb_pos + thumb_size {
                    cell.set_char('━');
                    cell.set_fg(Color::Rgb(140, 145, 160));
                } else {
                    cell.set_char('─');
                    cell.set_fg(Color::Rgb(35, 38, 46));
                }
            }
        }
    }

    /// Get (category, title, dsl_src) for the given global index into the
    /// combined static + procedural effect list.
    fn dsl_entry_info(global_index: usize) -> (String, String, String) {
        if global_index < DSL_SHOWCASE.len() {
            let e = &DSL_SHOWCASE[global_index];
            (
                e.category.to_string(),
                e.title.to_string(),
                e.dsl.to_string(),
            )
        } else {
            procedural_dsl_entry(global_index - DSL_SHOWCASE.len())
        }
    }

    /// Ensure the DSL effects cache has an entry at `index`, compiling on
    /// demand. Returns true if an effect exists at that slot.
    fn ensure_dsl_effect(&mut self, index: usize) -> bool {
        // Grow cache if needed
        if index >= self.dsl_effects_cache.len() {
            self.dsl_effects_cache
                .resize_with(index + 1, || None);
        }
        if self.dsl_effects_cache[index].is_none() {
            let (_cat, _title, dsl_src) = Self::dsl_entry_info(index);
            self.dsl_effects_cache[index] = compile_dsl_effect(&dsl_src);
        }
        self.dsl_effects_cache[index].is_some()
    }

    fn render_effects(&mut self, frame: &mut Frame, area: Rect) {
        let elapsed = self.frame_elapsed;
        let total = total_dsl_effects();

        // ── outer border ────────────────────────────────────────────────
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(55, 60, 70))
            .title(
                " TachyonFX DSL Showcase "
                    .bold()
                    .fg(Color::Rgb(207, 181, 59)),
            );
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 4 || inner.width < 10 {
            return;
        }

        // ── layout: scrollable entries + nav bar ────────────────────────
        let [entries_area, nav_bar] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)])
                .areas(inner);

        // Height of each effect entry: 3-row demo + title line + 1 blank
        let entry_height: u16 = 5;
        let visible_slots =
            (entries_area.height / entry_height).max(1) as usize;
        let max_scroll = total.saturating_sub(visible_slots);
        self.dsl_effects_scroll = self.dsl_effects_scroll.min(max_scroll);

        let start_idx = self.dsl_effects_scroll;
        let end_idx = (start_idx + visible_slots).min(total);

        // ── pre-compile visible effects ─────────────────────────────────
        for idx in start_idx..end_idx {
            self.ensure_dsl_effect(idx);
        }

        // ── render each visible entry ───────────────────────────────────
        for (slot, idx) in (start_idx..end_idx).enumerate() {
            let slot_y = entries_area.y + (slot as u16) * entry_height;
            if slot_y + entry_height > entries_area.bottom() {
                break;
            }

            let (category, title, dsl_src) = Self::dsl_entry_info(idx);

            // Title row
            let title_area = Rect::new(
                entries_area.x,
                slot_y,
                entries_area.width,
                1,
            );
            let label = format!(
                " {:>3}. [{}] {}",
                idx + 1,
                category,
                title,
            );
            frame.render_widget(
                Paragraph::new(label)
                    .style(Style::default().fg(Color::Rgb(207, 181, 59)).bold()),
                title_area,
            );

            // DSL code hint (first meaningful line, truncated)
            let code_line = dsl_src
                .lines()
                .map(|l| l.trim())
                .find(|l| !l.is_empty())
                .unwrap_or("...");
            let code_area = Rect::new(
                entries_area.x + 1,
                slot_y + 1,
                entries_area.width.saturating_sub(2),
                1,
            );
            let max_chars = code_area.width.saturating_sub(2) as usize;
            let code_display = if code_line.chars().count() > code_area.width as usize {
                let truncated: String = code_line.chars().take(max_chars).collect();
                format!("{}…", truncated)
            } else {
                code_line.to_string()
            };
            frame.render_widget(
                Paragraph::new(code_display)
                    .style(Style::default().fg(Color::Rgb(120, 125, 140))),
                code_area,
            );

            // Demo area (3 rows with sample text)
            let demo_area = Rect::new(
                entries_area.x + 1,
                slot_y + 2,
                entries_area.width.saturating_sub(2),
                2,
            );
            let sample = format!("│ {} │", title);
            frame.render_widget(
                Paragraph::new(vec![
                    Line::styled(
                        &sample,
                        Style::default().fg(Color::Rgb(220, 225, 235)).bold(),
                    ),
                    Line::styled(
                        "─".repeat(demo_area.width as usize),
                        Style::default().fg(Color::Rgb(55, 60, 70)),
                    ),
                ]),
                demo_area,
            );

            // Apply compiled effect
            if let Some(Some(ref mut effect)) = self.dsl_effects_cache.get_mut(idx) {
                frame.render_effect(effect, demo_area, elapsed);
            }
        }

        // ── nav bar ─────────────────────────────────────────────────────
        self.render_scroll_arrows(
            frame,
            nav_bar,
            self.dsl_effects_scroll,
            max_scroll,
            "swipe ↕ │ ◄► page",
        );
    }

    fn render_clock(&self, frame: &mut Frame, area: Rect) {
        // Get current time from browser via js_sys
        let date = web_sys::js_sys::Date::new_0();
        let hours = date.get_hours();
        let minutes = date.get_minutes();
        let seconds = date.get_seconds();

        let time_str = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);

        // Large ASCII digit font (3x5 per digit)
        const DIGITS: [&[&str; 5]; 10] = [
            &[" ▄▄ ", "█  █", "█  █", "█  █", " ▀▀ "], // 0
            &["  █ ", " ██ ", "  █ ", "  █ ", " ▄█▄"], // 1
            &[" ▄▄ ", "   █", " ▄▄ ", "█   ", " ▀▀▀"], // 2
            &[" ▄▄ ", "   █", " ▄▄ ", "   █", " ▀▀ "], // 3
            &["█  █", "█  █", " ▀▀█", "   █", "   ▀"], // 4
            &[" ▀▀▀", "█   ", " ▀▀ ", "   █", " ▀▀ "], // 5
            &[" ▄▄ ", "█   ", "█▄▄ ", "█  █", " ▀▀ "], // 6
            &[" ▀▀▀", "   █", "  █ ", " █  ", " ▀  "], // 7
            &[" ▄▄ ", "█  █", " ▄▄ ", "█  █", " ▀▀ "], // 8
            &[" ▄▄ ", "█  █", " ▀▀█", "   █", " ▀▀ "], // 9
        ];
        const COLON: [&str; 5] = ["  ", "▄ ", "  ", "▄ ", "  "];

        // Build 5 rows of the big clock display
        let mut big_lines: Vec<String> = vec![String::new(); 5];
        for ch in time_str.chars() {
            if ch == ':' {
                for (row, line) in big_lines.iter_mut().enumerate() {
                    line.push_str(COLON[row]);
                }
            } else if let Some(d) = ch.to_digit(10) {
                let glyph = DIGITS[d as usize];
                for (row, line) in big_lines.iter_mut().enumerate() {
                    line.push_str(glyph[row]);
                    line.push(' ');
                }
            }
        }

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(""));

        // Color cycle based on bg_tick for a subtle animated effect
        let hue = (self.bg_tick % 360) as f64;
        let r = (128.0 + 80.0 * (hue * std::f64::consts::PI / 180.0).sin()) as u8;
        let g = (128.0 + 80.0 * ((hue + 120.0) * std::f64::consts::PI / 180.0).sin()) as u8;
        let b = (128.0 + 80.0 * ((hue + 240.0) * std::f64::consts::PI / 180.0).sin()) as u8;

        for big_line in &big_lines {
            lines.push(Line::styled(
                big_line.clone(),
                Style::default().fg(Color::Rgb(r, g, b)).bold(),
            ));
        }

        lines.push(Line::from(""));

        // Date display
        const DAY_NAMES: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        const MONTH_NAMES: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
                           "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
        let day_of_week = date.get_day() as usize;
        let month = date.get_month() as usize;
        let day = date.get_date();
        let year = date.get_full_year();

        let date_str = format!(
            "{}  {} {} {}",
            DAY_NAMES.get(day_of_week).unwrap_or(&"???"),
            MONTH_NAMES.get(month).unwrap_or(&"???"),
            day,
            year
        );
        lines.push(Line::styled(
            date_str,
            Style::default().fg(Color::Rgb(207, 181, 59)).bold(),
        ));
        lines.push(Line::from(""));

        // UTC offset
        let tz_offset = date.get_timezone_offset();
        let tz_hours = -(tz_offset as i32) / 60;
        let tz_mins = ((tz_offset as i32).unsigned_abs()) % 60;
        let tz_str = format!("UTC {:+}:{:02}", tz_hours, tz_mins);
        lines.push(Line::styled(
            tz_str,
            Style::default().fg(Color::Rgb(140, 145, 155)),
        ));
        lines.push(Line::from(""));

        // Unix timestamp
        let unix_ms = date.get_time() as u64;
        lines.push(Line::styled(
            format!("Unix: {}", unix_ms / 1000),
            Style::default().fg(Color::Rgb(100, 105, 115)),
        ));

        let text = Text::from(lines);
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(55, 60, 70))
            .title(Line::from(" Clock ").alignment(Alignment::Center))
            .title_style(Style::default().fg(Color::Rgb(207, 181, 59)).bold());

        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Center)
            .block(block);

        frame.render_widget(paragraph, area);
    }

    fn render_matrix(&mut self, frame: &mut Frame, area: Rect) {
        const MATRIX_SPECIAL: [char; 10] = ['@', '#', '$', '%', '&', '*', '+', '=', '~', '^'];

        fn matrix_char(seed: u64) -> char {
            match seed % 3 {
                0 => (0x30 + (seed.wrapping_mul(7) % 10) as u8) as char, // 0-9
                1 => (0x41 + (seed.wrapping_mul(11) % 26) as u8) as char, // A-Z
                _ => MATRIX_SPECIAL[(seed % 10) as usize],
            }
        }

        let width = area.width.saturating_sub(2) as usize;
        let height = area.height.saturating_sub(2) as usize;

        if width == 0 || height == 0 {
            return;
        }

        // Initialize or resize matrix columns
        if !self.matrix_initialized || self.matrix_columns.len() != width {
            self.matrix_columns.clear();
            for i in 0..width {
                let seed = (i as u64).wrapping_mul(2654435761) ^ self.bg_tick;
                let speed = 1 + (seed % 3) as u16;
                let length = 4 + (seed.wrapping_mul(7) % 12) as u16;
                let head = (seed.wrapping_mul(13) % height as u64) as u16;
                let mut chars = Vec::new();
                for j in 0..length {
                    let ch_seed = seed.wrapping_mul(31).wrapping_add(j as u64);
                    chars.push(matrix_char(ch_seed));
                }
                self.matrix_columns.push(MatrixColumn {
                    head,
                    length,
                    speed,
                    counter: 0,
                    chars,
                });
            }
            self.matrix_initialized = true;
        }

        // Update column positions
        for (i, col) in self.matrix_columns.iter_mut().enumerate() {
            col.counter += 1;
            if col.counter >= col.speed {
                col.counter = 0;
                col.head += 1;
                if col.head > height as u16 + col.length {
                    col.head = 0;
                    // Regenerate chars for variety
                    let seed = (i as u64).wrapping_mul(2654435761) ^ self.bg_tick;
                    col.length = 4 + (seed.wrapping_mul(7) % 12) as u16;
                    col.speed = 1 + (seed % 3) as u16;
                    col.chars.clear();
                    for j in 0..col.length {
                        let ch_seed = seed.wrapping_mul(31).wrapping_add(j as u64);
                        col.chars.push(matrix_char(ch_seed));
                    }
                }
            }
        }

        // Render block border
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(55, 60, 70))
            .title(Line::from(" Matrix Rain ").alignment(Alignment::Center))
            .title_style(Style::default().fg(Color::Rgb(0, 255, 65)).bold());
        frame.render_widget(block, area);

        // Render matrix rain into the inner area
        let inner = Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), area.height.saturating_sub(2));
        let buf = frame.buffer_mut();

        for (col_idx, col) in self.matrix_columns.iter().enumerate() {
            if col_idx >= inner.width as usize {
                break;
            }
            let x = inner.x + col_idx as u16;
            for trail_pos in 0..col.length {
                let row = col.head as i32 - trail_pos as i32;
                if row < 0 || row >= inner.height as i32 {
                    continue;
                }
                let y = inner.y + row as u16;
                let pos = Position::new(x, y);
                if let Some(cell) = buf.cell_mut(pos) {
                    let ch = col.chars.get(trail_pos as usize).copied().unwrap_or('0');
                    cell.set_char(ch);
                    if trail_pos == 0 {
                        // Head: bright white-green
                        cell.set_fg(Color::Rgb(200, 255, 200));
                        cell.set_bg(Color::Rgb(0, 40, 0));
                    } else {
                        // Trail: fade from green to dark green
                        let fade = 255 - (trail_pos as u16 * 255 / col.length as u16).min(255);
                        let g = (fade as u8).max(30);
                        let r = (fade as u8 / 8).min(20);
                        cell.set_fg(Color::Rgb(r, g, 0));
                        cell.set_bg(Color::Rgb(0, (g / 12).min(15), 0));
                    }
                }
            }
        }
    }
}

fn open_url(url: &str) {
    // Defer opening the URL to avoid RefCell double-borrow.
    // window.open() can trigger synchronous browser events (focus, blur)
    // that re-enter draw() while handle_mouse_event() still holds borrow_mut().
    // Using setTimeout(0) ensures the URL opens after the current borrow is released.
    let escaped = url
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    let js = format!("setTimeout(function(){{window.open('{}','_blank','noopener')}},500)", escaped);
    let _ = web_sys::js_sys::eval(&js);
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
