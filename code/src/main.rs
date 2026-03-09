use std::cell::RefCell;
use std::rc::Rc;

use grift::Lisp;
use ratzilla::backend::webgl2::WebGl2BackendOptions;
use ratzilla::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratzilla::ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratzilla::ratatui::style::{Color, Modifier, Style, Stylize};
use ratzilla::ratatui::text::{Line, Span, Text};
use ratzilla::ratatui::widgets::{Block, BorderType, Paragraph, Wrap};
use ratzilla::ratatui::Frame;
use ratzilla::WebGl2Backend;
use ratzilla::WebRenderer;
use unicode_width::UnicodeWidthChar;

use std::collections::HashSet;
use tvk::layout::lisp_keyboard_layout;
use tvk::virtual_key::VirtualKey;

use md_tui::nodes::root::Component;
use md_tui::nodes::textcomponent::TextNode;
use md_tui::nodes::word::WordType;

use tachyonfx::dsl::EffectDsl;
use tachyonfx::fx::{self};
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

// ---------------------------------------------------------------------------
// Expanded Effects DSL Showcase
// ---------------------------------------------------------------------------
// Each entry: (category, title, DSL expression string).
// The DSL expressions are compiled at runtime by tachyonfx::dsl::EffectDsl.
// Effects are wrapped with repeating(sequence(ping_pong(...), sleep(1500)))
// so they loop forever with a readable pause between repetitions.
// ---------------------------------------------------------------------------

struct DslShowcaseEntry {
    category: &'static str,
    title: &'static str,
    dsl: &'static str,
}

#[derive(Clone, Copy)]
struct SectionStyle {
    heading: Color,
    body: Color,
}

impl SectionStyle {
    const fn new(heading: Color, body: Color) -> Self {
        Self { heading, body }
    }
}

const BODY_TEXT_COLOR: Color = Color::Rgb(170, 175, 185);
const SUBTLE_TEXT_COLOR: Color = Color::Rgb(110, 115, 125);
const MUTED_TEXT_COLOR: Color = Color::Rgb(75, 80, 90);
const LINK_TEXT_COLOR: Color = Color::Rgb(160, 175, 195);
const GOLD: Color = Color::Rgb(207, 181, 59);
const COPPER: Color = Color::Rgb(184, 115, 51);
const SILVER: Color = Color::Rgb(200, 200, 210);

fn push_blank_line(lines: &mut Vec<Line<'static>>) {
    lines.push(Line::from(""));
}

fn push_styled_multiline(lines: &mut Vec<Line<'static>>, text: &str, style: Style) {
    for line in text.lines() {
        lines.push(Line::styled(line.to_owned(), style));
    }
}

fn push_section(lines: &mut Vec<Line<'static>>, title: &str, body: &str, style: SectionStyle) {
    lines.push(Line::styled(
        title.to_owned(),
        Style::default().fg(style.heading).bold(),
    ));
    push_blank_line(lines);
    push_styled_multiline(lines, body, Style::default().fg(style.body));
    push_blank_line(lines);
}

fn push_bullet_list(lines: &mut Vec<Line<'static>>, items: &[&str], color: Color) {
    for item in items {
        lines.push(Line::styled((*item).to_owned(), Style::default().fg(color)));
    }
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
        title: "dissolve_to amber → coalesce",
        dsl: r#"
            fx::sequence(&[
                fx::dissolve_to(Style::default().fg(Color::Rgb(207, 181, 59)), (2000, QuadOut)),
                fx::coalesce((2000, SineOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Dissolve & Coalesce",
        title: "dissolve → coalesce_from teal",
        dsl: r#"
            fx::sequence(&[
                fx::dissolve((2000, QuadOut)),
                fx::coalesce_from(Style::default().fg(Color::Rgb(0, 180, 180)), (2000, CubicOut))
            ])
        "#,
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
        dsl: "fx::explode(1.0, 0.5, (3000, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Explosion & Motion",
        title: "stretch L→R",
        dsl: "fx::stretch(Motion::LeftToRight, Style::default(), (3000, CubicOut))",
    },
    DslShowcaseEntry {
        category: "Explosion & Motion",
        title: "stretch U→D",
        dsl: "fx::stretch(Motion::UpToDown, Style::default(), (3000, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Explosion & Motion",
        title: "expand Horizontal",
        dsl: "fx::expand(ExpandDirection::Horizontal, Style::default(), (3000, CubicOut))",
    },
    DslShowcaseEntry {
        category: "Explosion & Motion",
        title: "expand Vertical",
        dsl: "fx::expand(ExpandDirection::Vertical, Style::default(), (3000, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Explosion & Motion",
        title: "translate",
        dsl: "fx::translate(fx::consume_tick(), Offset { x: 3, y: 1 }, (2500, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Explosion & Motion",
        title: "translate reverse",
        dsl: "fx::translate(fx::consume_tick(), Offset { x: -3, y: -1 }, (2500, CubicOut))",
    },
    DslShowcaseEntry {
        category: "Explosion & Motion",
        title: "explode (BounceOut)",
        dsl: "fx::explode(1.0, 0.5, (3500, BounceOut))",
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
                .with_pattern(SweepPattern::left_to_right(5))
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
        dsl: r#"fx::evolve_into(EvolveSymbolSet::Circles, (3000, CubicOut))"#,
    },
    DslShowcaseEntry {
        category: "Evolution",
        title: "evolve_from Squares",
        dsl: r#"fx::evolve_from(EvolveSymbolSet::Squares, (3000, QuadOut))"#,
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
    // ── Diagonal Pattern Showcase ────────────────────────────────────────
    DslShowcaseEntry {
        category: "Diagonal Patterns",
        title: "dissolve TL→BR",
        dsl: r#"
            fx::dissolve((3500, QuadOut))
                .with_pattern(DiagonalPattern::top_left_to_bottom_right())
        "#,
    },
    DslShowcaseEntry {
        category: "Diagonal Patterns",
        title: "dissolve TR→BL",
        dsl: r#"
            fx::dissolve((3500, SineOut))
                .with_pattern(DiagonalPattern::top_right_to_bottom_left())
        "#,
    },
    DslShowcaseEntry {
        category: "Diagonal Patterns",
        title: "dissolve BL→TR",
        dsl: r#"
            fx::dissolve((3500, CubicOut))
                .with_pattern(DiagonalPattern::bottom_left_to_top_right())
        "#,
    },
    DslShowcaseEntry {
        category: "Diagonal Patterns",
        title: "dissolve BR→TL",
        dsl: r#"
            fx::dissolve((3500, QuadOut))
                .with_pattern(DiagonalPattern::bottom_right_to_top_left())
        "#,
    },
    DslShowcaseEntry {
        category: "Diagonal Patterns",
        title: "coalesce TL→BR",
        dsl: r#"
            fx::coalesce((3500, SineOut))
                .with_pattern(DiagonalPattern::top_left_to_bottom_right())
        "#,
    },
    DslShowcaseEntry {
        category: "Diagonal Patterns",
        title: "coalesce BR→TL",
        dsl: r#"
            fx::coalesce((3500, CubicOut))
                .with_pattern(DiagonalPattern::bottom_right_to_top_left())
        "#,
    },
    // ── Sweep Pattern Directions ─────────────────────────────────────────
    DslShowcaseEntry {
        category: "Sweep Patterns",
        title: "dissolve + sweep R→L",
        dsl: r#"
            fx::dissolve((3000, QuadOut))
                .with_pattern(SweepPattern::right_to_left(5))
        "#,
    },
    DslShowcaseEntry {
        category: "Sweep Patterns",
        title: "dissolve + sweep T→B",
        dsl: r#"
            fx::dissolve((3000, SineOut))
                .with_pattern(SweepPattern::up_to_down(5))
        "#,
    },
    DslShowcaseEntry {
        category: "Sweep Patterns",
        title: "dissolve + sweep B→T",
        dsl: r#"
            fx::dissolve((3000, CubicOut))
                .with_pattern(SweepPattern::down_to_up(5))
        "#,
    },
    DslShowcaseEntry {
        category: "Sweep Patterns",
        title: "coalesce + sweep L→R",
        dsl: r#"
            fx::coalesce((3000, QuadOut))
                .with_pattern(SweepPattern::left_to_right(5))
        "#,
    },
    DslShowcaseEntry {
        category: "Sweep Patterns",
        title: "coalesce + sweep R→L",
        dsl: r#"
            fx::coalesce((3000, SineOut))
                .with_pattern(SweepPattern::right_to_left(5))
        "#,
    },
    DslShowcaseEntry {
        category: "Sweep Patterns",
        title: "coalesce + sweep T→B",
        dsl: r#"
            fx::coalesce((3500, CubicOut))
                .with_pattern(SweepPattern::up_to_down(5))
        "#,
    },
    // ── Checkerboard Variations ──────────────────────────────────────────
    DslShowcaseEntry {
        category: "Checkerboard",
        title: "coalesce + checkerboard",
        dsl: r#"
            fx::coalesce((3500, QuadOut))
                .with_pattern(CheckerboardPattern::default())
        "#,
    },
    DslShowcaseEntry {
        category: "Checkerboard",
        title: "dissolve + checkerboard size 2",
        dsl: r#"
            fx::dissolve((3500, SineOut))
                .with_pattern(CheckerboardPattern::with_cell_size(2))
        "#,
    },
    DslShowcaseEntry {
        category: "Checkerboard",
        title: "coalesce + checkerboard size 3",
        dsl: r#"
            fx::coalesce((3500, CubicOut))
                .with_pattern(CheckerboardPattern::with_cell_size(3))
        "#,
    },
    DslShowcaseEntry {
        category: "Checkerboard",
        title: "dissolve + checkerboard size 4",
        dsl: r#"
            fx::dissolve((4000, QuadOut))
                .with_pattern(CheckerboardPattern::with_cell_size(4))
        "#,
    },
    // ── Spiral Variations ────────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Spiral Patterns",
        title: "dissolve + spiral 2 arms",
        dsl: r#"
            fx::dissolve((3500, QuadOut))
                .with_pattern(SpiralPattern::center().with_arms(2))
        "#,
    },
    DslShowcaseEntry {
        category: "Spiral Patterns",
        title: "dissolve + spiral 3 arms",
        dsl: r#"
            fx::dissolve((3500, SineOut))
                .with_pattern(SpiralPattern::center().with_arms(3))
        "#,
    },
    DslShowcaseEntry {
        category: "Spiral Patterns",
        title: "dissolve + spiral 5 arms",
        dsl: r#"
            fx::dissolve((4000, CubicOut))
                .with_pattern(SpiralPattern::center().with_arms(5))
        "#,
    },
    DslShowcaseEntry {
        category: "Spiral Patterns",
        title: "dissolve + spiral 8 arms",
        dsl: r#"
            fx::dissolve((4000, QuadOut))
                .with_pattern(SpiralPattern::center().with_arms(8))
        "#,
    },
    DslShowcaseEntry {
        category: "Spiral Patterns",
        title: "coalesce + spiral 2 arms wide",
        dsl: r#"
            fx::coalesce((4000, SineOut))
                .with_pattern(
                    SpiralPattern::center()
                        .with_arms(2)
                        .with_transition_width(3.0)
                )
        "#,
    },
    DslShowcaseEntry {
        category: "Spiral Patterns",
        title: "coalesce + spiral 4 arms narrow",
        dsl: r#"
            fx::coalesce((3500, CubicOut))
                .with_pattern(
                    SpiralPattern::center()
                        .with_arms(4)
                        .with_transition_width(1.0)
                )
        "#,
    },
    DslShowcaseEntry {
        category: "Spiral Patterns",
        title: "dissolve + spiral 7 arms",
        dsl: r#"
            fx::dissolve((4500, Linear))
                .with_pattern(SpiralPattern::center().with_arms(7))
        "#,
    },
    // ── Combined Pattern Showcase ────────────────────────────────────────
    DslShowcaseEntry {
        category: "Combined Patterns",
        title: "multiply radial × spiral",
        dsl: r#"
            fx::dissolve((4000, QuadOut))
                .with_pattern(CombinedPattern::multiply(
                    RadialPattern::center(),
                    SpiralPattern::center().with_arms(4)
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Combined Patterns",
        title: "max radial | diamond",
        dsl: r#"
            fx::dissolve((4000, SineOut))
                .with_pattern(CombinedPattern::max(
                    RadialPattern::center(),
                    DiamondPattern::center()
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Combined Patterns",
        title: "min radial & diamond",
        dsl: r#"
            fx::dissolve((4000, CubicOut))
                .with_pattern(CombinedPattern::min(
                    RadialPattern::center(),
                    DiamondPattern::center()
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Combined Patterns",
        title: "average spiral + sweep",
        dsl: r#"
            fx::dissolve((4000, QuadOut))
                .with_pattern(CombinedPattern::average(
                    SpiralPattern::center().with_arms(3),
                    SweepPattern::left_to_right(5)
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Combined Patterns",
        title: "multiply diagonal × checker",
        dsl: r#"
            fx::dissolve((4500, SineOut))
                .with_pattern(CombinedPattern::multiply(
                    DiagonalPattern::top_left_to_bottom_right(),
                    CheckerboardPattern::default()
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Combined Patterns",
        title: "max spiral | sweep",
        dsl: r#"
            fx::coalesce((4000, CubicOut))
                .with_pattern(CombinedPattern::max(
                    SpiralPattern::center().with_arms(5),
                    SweepPattern::up_to_down(5)
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Combined Patterns",
        title: "min checker & radial",
        dsl: r#"
            fx::coalesce((4000, QuadOut))
                .with_pattern(CombinedPattern::min(
                    CheckerboardPattern::default(),
                    RadialPattern::center()
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Combined Patterns",
        title: "average diamond + diagonal",
        dsl: r#"
            fx::dissolve((4500, SineOut))
                .with_pattern(CombinedPattern::average(
                    DiamondPattern::center(),
                    DiagonalPattern::bottom_left_to_top_right()
                ))
        "#,
    },
    // ── Inverted Patterns ────────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Inverted Patterns",
        title: "inverted diamond",
        dsl: r#"
            fx::dissolve((3500, QuadOut))
                .with_pattern(InvertedPattern::new(DiamondPattern::center()))
        "#,
    },
    DslShowcaseEntry {
        category: "Inverted Patterns",
        title: "inverted spiral",
        dsl: r#"
            fx::dissolve((4000, SineOut))
                .with_pattern(InvertedPattern::new(
                    SpiralPattern::center().with_arms(4)
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Inverted Patterns",
        title: "inverted diagonal",
        dsl: r#"
            fx::coalesce((3500, CubicOut))
                .with_pattern(InvertedPattern::new(
                    DiagonalPattern::top_left_to_bottom_right()
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Inverted Patterns",
        title: "inverted sweep L→R",
        dsl: r#"
            fx::dissolve((3500, QuadOut))
                .with_pattern(InvertedPattern::new(
                    SweepPattern::left_to_right(5)
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Inverted Patterns",
        title: "inverted checkerboard",
        dsl: r#"
            fx::coalesce((3500, SineOut))
                .with_pattern(InvertedPattern::new(
                    CheckerboardPattern::default()
                ))
        "#,
    },
    // ── Blend Patterns ───────────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Blend Patterns",
        title: "blend radial↔diamond",
        dsl: r#"
            fx::dissolve((4500, QuadOut))
                .with_pattern(BlendPattern::new(
                    RadialPattern::center(),
                    DiamondPattern::center()
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Blend Patterns",
        title: "blend sweep↔spiral",
        dsl: r#"
            fx::dissolve((4500, SineOut))
                .with_pattern(BlendPattern::new(
                    SweepPattern::left_to_right(5),
                    SpiralPattern::center().with_arms(3)
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Blend Patterns",
        title: "blend checker↔radial",
        dsl: r#"
            fx::coalesce((4500, CubicOut))
                .with_pattern(BlendPattern::new(
                    CheckerboardPattern::default(),
                    RadialPattern::center()
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Blend Patterns",
        title: "blend diagonal↔spiral",
        dsl: r#"
            fx::dissolve((5000, QuadOut))
                .with_pattern(BlendPattern::new(
                    DiagonalPattern::top_left_to_bottom_right(),
                    SpiralPattern::center().with_arms(5)
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Blend Patterns",
        title: "blend diamond↔sweep",
        dsl: r#"
            fx::coalesce((4500, SineOut))
                .with_pattern(BlendPattern::new(
                    DiamondPattern::center(),
                    SweepPattern::right_to_left(5)
                ))
        "#,
    },
    // ── Multi-Stage Sequences ────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Multi-Stage",
        title: "dissolve → fade → coalesce",
        dsl: r#"
            fx::sequence(&[
                fx::dissolve(1200),
                fx::fade_to_fg(Color::Rgb(207, 181, 59), 1200),
                fx::coalesce(1200)
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Multi-Stage",
        title: "sweep → dissolve → sweep back",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sequence(&[
                fx::sweep_in(Motion::LeftToRight, 10, 3, bg, (1200, QuadOut)),
                fx::dissolve(1200),
                fx::sweep_in(Motion::RightToLeft, 10, 3, bg, (1200, CubicOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Multi-Stage",
        title: "evolve → paint → dissolve",
        dsl: r#"
            fx::sequence(&[
                fx::evolve(EvolveSymbolSet::Shaded, (1200, QuadOut)),
                fx::paint_fg(Color::Rgb(184, 115, 51), 1200),
                fx::dissolve((1200, SineOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Multi-Stage",
        title: "4-color paint cascade",
        dsl: r#"
            fx::sequence(&[
                fx::paint_fg(Color::Rgb(220, 20, 60), 800),
                fx::paint_fg(Color::Rgb(255, 140, 0), 800),
                fx::paint_fg(Color::Rgb(207, 181, 59), 800),
                fx::paint_fg(Color::Rgb(50, 205, 50), 800),
                fx::paint_fg(Color::Rgb(100, 149, 237), 800)
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Multi-Stage",
        title: "5-fade rainbow chain",
        dsl: r#"
            fx::sequence(&[
                fx::fade_to_fg(Color::Red, 700),
                fx::fade_to_fg(Color::Yellow, 700),
                fx::fade_to_fg(Color::Green, 700),
                fx::fade_to_fg(Color::Cyan, 700),
                fx::fade_to_fg(Color::Blue, 700)
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Multi-Stage",
        title: "slide L → sleep → slide R",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sequence(&[
                fx::slide_in(Motion::LeftToRight, 8, 3, bg, (1200, QuadOut)),
                fx::sleep(500),
                fx::slide_in(Motion::RightToLeft, 8, 3, bg, (1200, CubicOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Multi-Stage",
        title: "dissolve → sleep → coalesce → sleep",
        dsl: r#"
            fx::sequence(&[
                fx::dissolve(1500),
                fx::sleep(300),
                fx::coalesce((1500, SineOut)),
                fx::sleep(300)
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Multi-Stage",
        title: "4-sweep pinwheel",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sequence(&[
                fx::sweep_in(Motion::LeftToRight, 10, 3, bg, (900, QuadOut)),
                fx::sweep_in(Motion::UpToDown, 8, 2, bg, (900, QuadOut)),
                fx::sweep_in(Motion::RightToLeft, 10, 3, bg, (900, QuadOut)),
                fx::sweep_in(Motion::DownToUp, 8, 2, bg, (900, CubicOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Multi-Stage",
        title: "hsl shift → saturate → lighten",
        dsl: r#"
            fx::sequence(&[
                fx::hsl_shift_fg([60.0, 20.0, 0.0], 1200),
                fx::saturate_fg(30.0, 1200),
                fx::lighten_fg(25.0, (1200, SineOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Multi-Stage",
        title: "evolve chain 3 styles",
        dsl: r#"
            fx::sequence(&[
                fx::evolve(EvolveSymbolSet::BlocksHorizontal, (1200, QuadOut)),
                fx::evolve(EvolveSymbolSet::Shaded, (1200, SineOut)),
                fx::evolve(EvolveSymbolSet::Quadrants, (1200, CubicOut))
            ])
        "#,
    },
    // ── Color Crossfade Showcase ─────────────────────────────────────────
    DslShowcaseEntry {
        category: "Color Crossfade",
        title: "fade_to red→blue",
        dsl: r#"fx::fade_to(Color::Red, Color::Blue, (3000, QuadOut))"#,
    },
    DslShowcaseEntry {
        category: "Color Crossfade",
        title: "fade_to cyan→magenta",
        dsl: r#"fx::fade_to(Color::Cyan, Color::Magenta, (3000, SineOut))"#,
    },
    DslShowcaseEntry {
        category: "Color Crossfade",
        title: "fade_to green→yellow",
        dsl: r#"fx::fade_to(Color::Green, Color::Yellow, (3000, CubicOut))"#,
    },
    DslShowcaseEntry {
        category: "Color Crossfade",
        title: "fade_from crimson",
        dsl: r#"fx::fade_from(Color::Rgb(220, 20, 60), Color::Rgb(220, 20, 60), (3000, SineOut))"#,
    },
    DslShowcaseEntry {
        category: "Color Crossfade",
        title: "fade_from violet",
        dsl: r#"fx::fade_from(Color::Rgb(138, 43, 226), Color::Rgb(138, 43, 226), (3000, CubicOut))"#,
    },
    DslShowcaseEntry {
        category: "Color Crossfade",
        title: "fade_from lime",
        dsl: r#"fx::fade_from(Color::Rgb(50, 205, 50), Color::Rgb(50, 205, 50), (3500, QuadOut))"#,
    },
    DslShowcaseEntry {
        category: "Color Crossfade",
        title: "fade_to gold→copper",
        dsl: r#"fx::fade_to(Color::Rgb(207, 181, 59), Color::Rgb(184, 115, 51), (3000, SineOut))"#,
    },
    DslShowcaseEntry {
        category: "Color Crossfade",
        title: "fade_from_fg teal",
        dsl: "fx::fade_from_fg(Color::Rgb(0, 180, 180), (3000, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Color Crossfade",
        title: "fade_from_fg gold",
        dsl: "fx::fade_from_fg(Color::Rgb(207, 181, 59), (3000, SineOut))",
    },
    DslShowcaseEntry {
        category: "Color Crossfade",
        title: "fade_to_fg violet (CubicInOut)",
        dsl: "fx::fade_to_fg(Color::Rgb(138, 43, 226), (3000, CubicInOut))",
    },
    // ── Direction Showcase ───────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Direction Showcase",
        title: "stretch R→L",
        dsl: "fx::stretch(Motion::RightToLeft, Style::default(), (3000, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Direction Showcase",
        title: "stretch D→U",
        dsl: "fx::stretch(Motion::DownToUp, Style::default(), (3000, SineOut))",
    },
    DslShowcaseEntry {
        category: "Direction Showcase",
        title: "expand Horizontal",
        dsl: "fx::expand(ExpandDirection::Horizontal, Style::default(), (3000, CubicOut))",
    },
    DslShowcaseEntry {
        category: "Direction Showcase",
        title: "expand Vertical",
        dsl: "fx::expand(ExpandDirection::Vertical, Style::default(), (3000, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Direction Showcase",
        title: "sweep_out R→L",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sweep_out(Motion::RightToLeft, 10, 3, bg, (3000, QuadOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Direction Showcase",
        title: "sweep_out U→D",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sweep_out(Motion::UpToDown, 8, 2, bg, (3000, SineOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Direction Showcase",
        title: "sweep_out D→U",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sweep_out(Motion::DownToUp, 8, 2, bg, (3000, CubicOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Direction Showcase",
        title: "slide_out L→R",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::slide_out(Motion::LeftToRight, 8, 3, bg, (3000, QuadOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Direction Showcase",
        title: "slide_out U→D",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::slide_out(Motion::UpToDown, 8, 3, bg, (3000, SineOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Direction Showcase",
        title: "slide_out D→U",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::slide_out(Motion::DownToUp, 8, 3, bg, (3000, CubicOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Direction Showcase",
        title: "translate diagonal +5,+2",
        dsl: "fx::translate(fx::consume_tick(), Offset { x: 5, y: 2 }, (3000, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Direction Showcase",
        title: "translate diagonal -5,-2",
        dsl: "fx::translate(fx::consume_tick(), Offset { x: -5, y: -2 }, (3000, SineOut))",
    },
    DslShowcaseEntry {
        category: "Direction Showcase",
        title: "translate horizontal +8",
        dsl: "fx::translate(fx::consume_tick(), Offset { x: 8, y: 0 }, (3000, CubicOut))",
    },
    DslShowcaseEntry {
        category: "Direction Showcase",
        title: "translate vertical +4",
        dsl: "fx::translate(fx::consume_tick(), Offset { x: 0, y: 4 }, (3000, BounceOut))",
    },
    // ── Advanced HSL Combinations ────────────────────────────────────────
    DslShowcaseEntry {
        category: "HSL Advanced",
        title: "hsl_shift_fg deep blue shift",
        dsl: "fx::hsl_shift_fg([-60.0, 30.0, -20.0], (4000, QuadOut))",
    },
    DslShowcaseEntry {
        category: "HSL Advanced",
        title: "hsl_shift_fg golden warm",
        dsl: "fx::hsl_shift_fg([40.0, 35.0, 20.0], (3500, SineOut))",
    },
    DslShowcaseEntry {
        category: "HSL Advanced",
        title: "hsl_shift_fg ice cold",
        dsl: "fx::hsl_shift_fg([-90.0, 20.0, 30.0], (3500, CubicOut))",
    },
    DslShowcaseEntry {
        category: "HSL Advanced",
        title: "saturate_fg intense",
        dsl: "fx::saturate_fg(80.0, (3000, QuadOut))",
    },
    DslShowcaseEntry {
        category: "HSL Advanced",
        title: "lighten_fg bright",
        dsl: "fx::lighten_fg(60.0, (3000, SineOut))",
    },
    DslShowcaseEntry {
        category: "HSL Advanced",
        title: "darken_fg deep",
        dsl: "fx::darken_fg(60.0, (3000, CubicOut))",
    },
    DslShowcaseEntry {
        category: "HSL Advanced",
        title: "saturate + hsl shift",
        dsl: r#"
            fx::parallel(&[
                fx::saturate_fg(40.0, (3500, QuadOut)),
                fx::hsl_shift_fg([30.0, 0.0, 0.0], (3500, SineOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "HSL Advanced",
        title: "lighten + hsl shift",
        dsl: r#"
            fx::parallel(&[
                fx::lighten_fg(30.0, (3500, SineOut)),
                fx::hsl_shift_fg([90.0, 0.0, 0.0], (3500, Linear))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "HSL Advanced",
        title: "darken + saturate",
        dsl: r#"
            fx::parallel(&[
                fx::darken_fg(25.0, (3500, QuadOut)),
                fx::saturate_fg(35.0, (3500, CubicOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "HSL Advanced",
        title: "triple hsl: shift+sat+light",
        dsl: r#"
            fx::parallel(&[
                fx::hsl_shift_fg([45.0, 0.0, 0.0], (4000, Linear)),
                fx::saturate_fg(25.0, (4000, SineOut)),
                fx::lighten_fg(15.0, (4000, QuadOut))
            ])
        "#,
    },
    // ── Filter Combinations ──────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Filter Combos",
        title: "coalesce Text only",
        dsl: r#"
            fx::coalesce((3000, QuadOut))
                .with_filter(CellFilter::Text)
        "#,
    },
    DslShowcaseEntry {
        category: "Filter Combos",
        title: "fade_to_fg Text + gold",
        dsl: r#"
            fx::fade_to_fg(Color::Rgb(207, 181, 59), (3000, SineOut))
                .with_filter(CellFilter::Text)
        "#,
    },
    DslShowcaseEntry {
        category: "Filter Combos",
        title: "dissolve NonEmpty",
        dsl: r#"
            fx::dissolve((3000, CubicOut))
                .with_filter(CellFilter::NonEmpty)
        "#,
    },
    DslShowcaseEntry {
        category: "Filter Combos",
        title: "coalesce NonEmpty",
        dsl: r#"
            fx::coalesce((3000, QuadOut))
                .with_filter(CellFilter::NonEmpty)
        "#,
    },
    DslShowcaseEntry {
        category: "Filter Combos",
        title: "paint_fg Text + copper",
        dsl: r#"
            fx::paint_fg(Color::Rgb(184, 115, 51), (3000, SineOut))
                .with_filter(CellFilter::Text)
        "#,
    },
    DslShowcaseEntry {
        category: "Filter Combos",
        title: "hsl_shift NonEmpty",
        dsl: r#"
            fx::hsl_shift_fg([60.0, 30.0, 15.0], (3500, QuadOut))
                .with_filter(CellFilter::NonEmpty)
        "#,
    },
    DslShowcaseEntry {
        category: "Filter Combos",
        title: "evolve Text filter",
        dsl: r#"
            fx::evolve(EvolveSymbolSet::Shaded, (3000, QuadOut))
                .with_filter(CellFilter::Text)
        "#,
    },
    DslShowcaseEntry {
        category: "Filter Combos",
        title: "dissolve Text + radial",
        dsl: r#"
            fx::dissolve((3500, QuadOut))
                .with_filter(CellFilter::Text)
                .with_pattern(RadialPattern::center())
        "#,
    },
    // ── Advanced Wave Patterns ───────────────────────────────────────────
    DslShowcaseEntry {
        category: "Advanced Waves",
        title: "wave sin fast horizontal",
        dsl: r#"
            fx::dissolve((4000, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(Oscillator::sin(4.0, 0.0, 2.0))
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced Waves",
        title: "wave cos vertical",
        dsl: r#"
            fx::dissolve((4000, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(Oscillator::cos(0.0, 4.0, 1.5))
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced Waves",
        title: "wave sin diagonal",
        dsl: r#"
            fx::dissolve((4500, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(Oscillator::sin(2.0, 2.0, 1.0))
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced Waves",
        title: "wave triangle fast",
        dsl: r#"
            fx::coalesce((4000, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(Oscillator::triangle(3.0, 0.0, 1.5))
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced Waves",
        title: "wave sawtooth diagonal",
        dsl: r#"
            fx::dissolve((4500, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(Oscillator::sawtooth(2.0, 2.0, 1.0))
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced Waves",
        title: "wave sin phase shift",
        dsl: r#"
            fx::dissolve((4500, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(
                        Oscillator::sin(2.0, 0.0, 1.0).phase(1.57)
                    )
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced Waves",
        title: "wave modulated FM",
        dsl: r#"
            fx::dissolve((5000, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(
                        Oscillator::sin(3.0, 0.0, 1.0)
                            .modulated_by(
                                Modulator::cos(0.0, 2.0, 0.5)
                                    .intensity(0.8)
                                    .on_phase()
                            )
                    )
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced Waves",
        title: "wave modulated AM",
        dsl: r#"
            fx::coalesce((5000, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(
                        Oscillator::cos(2.0, 0.0, 0.8)
                            .modulated_by(
                                Modulator::sin(0.5, 0.5, 0.3)
                                    .intensity(0.6)
                                    .on_amplitude()
                            )
                    )
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced Waves",
        title: "wave multiply sin×cos",
        dsl: r#"
            fx::dissolve((5000, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(Oscillator::sin(3.0, 0.0, 1.0))
                        .multiply(Oscillator::cos(0.0, 2.0, 0.8))
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced Waves",
        title: "wave average sin+triangle",
        dsl: r#"
            fx::dissolve((5000, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(Oscillator::sin(2.0, 0.0, 1.0))
                        .average(Oscillator::triangle(0.0, 3.0, 0.5))
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced Waves",
        title: "wave max sin|sawtooth",
        dsl: r#"
            fx::coalesce((5000, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(Oscillator::sin(2.0, 1.0, 0.8))
                        .max(Oscillator::sawtooth(1.0, 0.0, 1.2))
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced Waves",
        title: "wave high contrast",
        dsl: r#"
            fx::dissolve((4000, Linear))
                .with_pattern(
                    WavePattern::new(
                        WaveLayer::new(Oscillator::sin(2.0, 0.0, 1.0))
                            .amplitude(1.0)
                    ).with_contrast(5)
                )
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced Waves",
        title: "wave low amplitude",
        dsl: r#"
            fx::dissolve((5000, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(Oscillator::sin(2.0, 0.0, 1.0))
                        .amplitude(0.4)
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced Waves",
        title: "wave power squared",
        dsl: r#"
            fx::dissolve((4500, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(Oscillator::sin(2.0, 0.0, 1.0))
                        .power(2)
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced Waves",
        title: "wave power cubed",
        dsl: r#"
            fx::coalesce((4500, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(Oscillator::cos(0.0, 3.0, 0.8))
                        .power(3)
                ))
        "#,
    },
    DslShowcaseEntry {
        category: "Advanced Waves",
        title: "wave abs value",
        dsl: r#"
            fx::dissolve((4500, Linear))
                .with_pattern(WavePattern::new(
                    WaveLayer::new(Oscillator::sin(2.0, 0.0, 1.0))
                        .abs()
                ))
        "#,
    },
    // ── Timing Chain Showcase ────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Timing Chains",
        title: "prolong_start 1s dissolve",
        dsl: "fx::prolong_start(1000, fx::dissolve((2000, QuadOut)))",
    },
    DslShowcaseEntry {
        category: "Timing Chains",
        title: "prolong_end 1s coalesce",
        dsl: "fx::prolong_end(1000, fx::coalesce((2000, SineOut)))",
    },
    DslShowcaseEntry {
        category: "Timing Chains",
        title: "with_duration 5s dissolve",
        dsl: "fx::with_duration(5000, fx::dissolve((2500, CubicOut)))",
    },
    DslShowcaseEntry {
        category: "Timing Chains",
        title: "delay 800ms + sweep",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sequence(&[
                fx::sleep(800),
                fx::sweep_in(Motion::LeftToRight, 10, 3, bg, (2500, QuadOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Timing Chains",
        title: "sleep → dissolve → sleep → coalesce",
        dsl: r#"
            fx::sequence(&[
                fx::sleep(400),
                fx::dissolve(1500),
                fx::sleep(400),
                fx::coalesce((1500, SineOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Timing Chains",
        title: "prolong_start sweep",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::prolong_start(600, fx::sweep_in(Motion::LeftToRight, 10, 3, bg, (2500, QuadOut)))
        "#,
    },
    DslShowcaseEntry {
        category: "Timing Chains",
        title: "prolong_end fade",
        dsl: "fx::prolong_end(600, fx::fade_to_fg(Color::Rgb(207, 181, 59), (2000, QuadOut)))",
    },
    // ── Interpolation Showcase Extended ──────────────────────────────────
    DslShowcaseEntry {
        category: "Interpolation Ext",
        title: "dissolve QuadInOut",
        dsl: "fx::dissolve((3000, QuadInOut))",
    },
    DslShowcaseEntry {
        category: "Interpolation Ext",
        title: "dissolve SineInOut",
        dsl: "fx::dissolve((3000, SineInOut))",
    },
    DslShowcaseEntry {
        category: "Interpolation Ext",
        title: "dissolve ExpoIn",
        dsl: "fx::dissolve((3000, ExpoIn))",
    },
    DslShowcaseEntry {
        category: "Interpolation Ext",
        title: "dissolve ExpoInOut",
        dsl: "fx::dissolve((3000, ExpoInOut))",
    },
    DslShowcaseEntry {
        category: "Interpolation Ext",
        title: "dissolve ElasticIn",
        dsl: "fx::dissolve((3000, ElasticIn))",
    },
    DslShowcaseEntry {
        category: "Interpolation Ext",
        title: "dissolve ElasticInOut",
        dsl: "fx::dissolve((3000, ElasticInOut))",
    },
    DslShowcaseEntry {
        category: "Interpolation Ext",
        title: "dissolve BounceInOut",
        dsl: "fx::dissolve((3000, BounceInOut))",
    },
    DslShowcaseEntry {
        category: "Interpolation Ext",
        title: "coalesce QuadInOut",
        dsl: "fx::coalesce((3000, QuadInOut))",
    },
    DslShowcaseEntry {
        category: "Interpolation Ext",
        title: "coalesce CubicInOut",
        dsl: "fx::coalesce((3000, CubicInOut))",
    },
    DslShowcaseEntry {
        category: "Interpolation Ext",
        title: "coalesce SineInOut",
        dsl: "fx::coalesce((3000, SineInOut))",
    },
    DslShowcaseEntry {
        category: "Interpolation Ext",
        title: "coalesce ExpoOut",
        dsl: "fx::coalesce((3000, ExpoOut))",
    },
    DslShowcaseEntry {
        category: "Interpolation Ext",
        title: "coalesce BounceOut",
        dsl: "fx::coalesce((3000, BounceOut))",
    },
    DslShowcaseEntry {
        category: "Interpolation Ext",
        title: "coalesce ElasticOut",
        dsl: "fx::coalesce((3000, ElasticOut))",
    },
    // ── Evolution Extended ───────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Evolution Ext",
        title: "evolve CircleFill",
        dsl: "fx::evolve(EvolveSymbolSet::CircleFill, (3000, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Evolution Ext",
        title: "evolve Squares (BounceOut)",
        dsl: "fx::evolve(EvolveSymbolSet::Squares, (3500, BounceOut))",
    },
    DslShowcaseEntry {
        category: "Evolution Ext",
        title: "evolve Circles (SineOut)",
        dsl: "fx::evolve(EvolveSymbolSet::Circles, (3000, SineOut))",
    },
    DslShowcaseEntry {
        category: "Evolution Ext",
        title: "evolve Shaded (CubicInOut)",
        dsl: "fx::evolve(EvolveSymbolSet::Shaded, (3000, CubicInOut))",
    },
    DslShowcaseEntry {
        category: "Evolution Ext",
        title: "evolve_into BlocksH ▓",
        dsl: r#"fx::evolve_into(EvolveSymbolSet::BlocksHorizontal, (3000, QuadOut))"#,
    },
    DslShowcaseEntry {
        category: "Evolution Ext",
        title: "evolve_from Shaded ░",
        dsl: r#"fx::evolve_from(EvolveSymbolSet::Shaded, (3000, SineOut))"#,
    },
    DslShowcaseEntry {
        category: "Evolution Ext",
        title: "evolve_into Quadrants ▙",
        dsl: r#"fx::evolve_into(EvolveSymbolSet::Quadrants, (3000, CubicOut))"#,
    },
    DslShowcaseEntry {
        category: "Evolution Ext",
        title: "evolve BlocksVertical slow",
        dsl: "fx::evolve(EvolveSymbolSet::BlocksVertical, (4000, Linear))",
    },
    // ── Paint Extended ───────────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Paint Extended",
        title: "paint_fg crimson",
        dsl: "fx::paint_fg(Color::Rgb(220, 20, 60), (2500, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Paint Extended",
        title: "paint_fg violet",
        dsl: "fx::paint_fg(Color::Rgb(138, 43, 226), (2500, SineOut))",
    },
    DslShowcaseEntry {
        category: "Paint Extended",
        title: "paint_fg lime",
        dsl: "fx::paint_fg(Color::Rgb(50, 205, 50), (2500, CubicOut))",
    },
    DslShowcaseEntry {
        category: "Paint Extended",
        title: "paint_fg cornflower",
        dsl: "fx::paint_fg(Color::Rgb(100, 149, 237), (2500, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Paint Extended",
        title: "paint_fg dark orange",
        dsl: "fx::paint_fg(Color::Rgb(255, 140, 0), (2500, SineOut))",
    },
    DslShowcaseEntry {
        category: "Paint Extended",
        title: "paint_fg white",
        dsl: "fx::paint_fg(Color::Rgb(240, 240, 245), (2500, CubicOut))",
    },
    // ── Radial Pattern Variations ────────────────────────────────────────
    DslShowcaseEntry {
        category: "Radial Variations",
        title: "radial wide transition",
        dsl: r#"
            fx::dissolve((4000, QuadOut))
                .with_pattern(
                    RadialPattern::center()
                        .with_transition_width(3.0)
                )
        "#,
    },
    DslShowcaseEntry {
        category: "Radial Variations",
        title: "radial narrow transition",
        dsl: r#"
            fx::dissolve((3500, SineOut))
                .with_pattern(
                    RadialPattern::center()
                        .with_transition_width(0.5)
                )
        "#,
    },
    DslShowcaseEntry {
        category: "Radial Variations",
        title: "radial off-center 0.2,0.3",
        dsl: r#"
            fx::dissolve((3500, CubicOut))
                .with_pattern(
                    RadialPattern::center()
                        .with_center(0.2, 0.3)
                )
        "#,
    },
    DslShowcaseEntry {
        category: "Radial Variations",
        title: "radial off-center 0.8,0.2",
        dsl: r#"
            fx::coalesce((3500, QuadOut))
                .with_pattern(
                    RadialPattern::center()
                        .with_center(0.8, 0.2)
                )
        "#,
    },
    DslShowcaseEntry {
        category: "Radial Variations",
        title: "radial corner 0.0,0.0",
        dsl: r#"
            fx::dissolve((4000, SineOut))
                .with_pattern(
                    RadialPattern::center()
                        .with_center(0.0, 0.0)
                )
        "#,
    },
    DslShowcaseEntry {
        category: "Radial Variations",
        title: "radial corner 1.0,1.0",
        dsl: r#"
            fx::coalesce((4000, CubicOut))
                .with_pattern(
                    RadialPattern::center()
                        .with_center(1.0, 1.0)
                )
        "#,
    },
    // ── Complex Parallel Compositions ────────────────────────────────────
    DslShowcaseEntry {
        category: "Complex Parallel",
        title: "dissolve + fade + saturate",
        dsl: r#"
            fx::parallel(&[
                fx::dissolve((4000, QuadOut))
                    .with_pattern(RadialPattern::center()),
                fx::fade_to_fg(Color::Rgb(207, 181, 59), (4000, SineOut)),
                fx::saturate_fg(30.0, (4000, CubicOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Complex Parallel",
        title: "coalesce + lighten + shift",
        dsl: r#"
            fx::parallel(&[
                fx::coalesce((4000, SineOut))
                    .with_pattern(DiamondPattern::center()),
                fx::lighten_fg(25.0, (4000, QuadOut)),
                fx::hsl_shift_fg([30.0, 10.0, 0.0], (4000, Linear))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Complex Parallel",
        title: "sweep + paint + darken",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::parallel(&[
                fx::sweep_in(Motion::LeftToRight, 10, 3, bg, (4000, QuadOut)),
                fx::paint_fg(Color::Rgb(0, 180, 180), (4000, SineOut)),
                fx::darken_fg(15.0, (4000, CubicOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Complex Parallel",
        title: "evolve + hsl + fade",
        dsl: r#"
            fx::parallel(&[
                fx::evolve(EvolveSymbolSet::Shaded, (4000, QuadOut)),
                fx::hsl_shift_fg([60.0, 20.0, 10.0], (4000, SineOut)),
                fx::fade_to_fg(Color::Rgb(184, 115, 51), (4000, CubicOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Complex Parallel",
        title: "explode + paint pink",
        dsl: r#"
            fx::parallel(&[
                fx::explode(1.0, 0.5, (3500, QuadOut)),
                fx::paint_fg(Color::Rgb(255, 105, 180), (3500, SineOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Complex Parallel",
        title: "stretch + fade amber",
        dsl: r#"
            fx::parallel(&[
                fx::stretch(Motion::LeftToRight, Style::default(), (3500, CubicOut)),
                fx::fade_to_fg(Color::Rgb(207, 181, 59), (3500, QuadOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Complex Parallel",
        title: "expand + hsl shift neon",
        dsl: r#"
            fx::parallel(&[
                fx::expand(ExpandDirection::Vertical, Style::default(), (3500, QuadOut)),
                fx::hsl_shift_fg([90.0, 40.0, 20.0], (3500, SineOut))
            ])
        "#,
    },
    // ── Complex Nested Compositions ──────────────────────────────────────
    DslShowcaseEntry {
        category: "Nested Compositions",
        title: "seq of parallels",
        dsl: r#"
            fx::sequence(&[
                fx::parallel(&[
                    fx::dissolve(1500),
                    fx::fade_to_fg(Color::Red, 1500)
                ]),
                fx::parallel(&[
                    fx::coalesce(1500),
                    fx::fade_to_fg(Color::Blue, 1500)
                ])
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Nested Compositions",
        title: "par sweeps + seq fades",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::parallel(&[
                fx::sweep_in(Motion::LeftToRight, 10, 3, bg, (4000, QuadOut)),
                fx::sequence(&[
                    fx::fade_to_fg(Color::Rgb(207, 181, 59), 2000),
                    fx::fade_to_fg(Color::Rgb(184, 115, 51), 2000)
                ])
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Nested Compositions",
        title: "seq dissolve + parallel shift",
        dsl: r#"
            fx::sequence(&[
                fx::dissolve((1500, QuadOut)),
                fx::parallel(&[
                    fx::coalesce((2000, SineOut)),
                    fx::hsl_shift_fg([45.0, 20.0, 10.0], (2000, CubicOut))
                ])
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Nested Compositions",
        title: "triple nested sequence",
        dsl: r#"
            fx::sequence(&[
                fx::dissolve(1000),
                fx::sequence(&[
                    fx::fade_to_fg(Color::Red, 800),
                    fx::fade_to_fg(Color::Green, 800)
                ]),
                fx::coalesce(1000)
            ])
        "#,
    },
    // ── Explode Variations ───────────────────────────────────────────────
    DslShowcaseEntry {
        category: "Explode Variations",
        title: "explode (QuadOut)",
        dsl: "fx::explode(1.0, 0.5, (3000, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Explode Variations",
        title: "explode (SineOut)",
        dsl: "fx::explode(1.0, 0.5, (3000, SineOut))",
    },
    DslShowcaseEntry {
        category: "Explode Variations",
        title: "explode (CubicOut)",
        dsl: "fx::explode(1.0, 0.5, (3000, CubicOut))",
    },
    DslShowcaseEntry {
        category: "Explode Variations",
        title: "explode (ElasticOut)",
        dsl: "fx::explode(1.0, 0.5, (4000, ElasticOut))",
    },
    DslShowcaseEntry {
        category: "Explode Variations",
        title: "explode (ExpoOut)",
        dsl: "fx::explode(1.0, 0.5, (3000, ExpoOut))",
    },
    DslShowcaseEntry {
        category: "Explode Variations",
        title: "explode fast",
        dsl: "fx::explode(1.0, 0.5, (1500, QuadOut))",
    },
    DslShowcaseEntry {
        category: "Explode Variations",
        title: "explode slow",
        dsl: "fx::explode(1.0, 0.5, (5000, Linear))",
    },
    // ── Color Space Comparisons ──────────────────────────────────────────
    DslShowcaseEntry {
        category: "Color Space Cmp",
        title: "fade red HSV",
        dsl: r#"
            fx::fade_to_fg(Color::Red, (3000, QuadOut))
                .with_color_space(ColorSpace::Hsv)
        "#,
    },
    DslShowcaseEntry {
        category: "Color Space Cmp",
        title: "fade red HSL",
        dsl: r#"
            fx::fade_to_fg(Color::Red, (3000, QuadOut))
                .with_color_space(ColorSpace::Hsl)
        "#,
    },
    DslShowcaseEntry {
        category: "Color Space Cmp",
        title: "fade red RGB",
        dsl: r#"
            fx::fade_to_fg(Color::Red, (3000, QuadOut))
                .with_color_space(ColorSpace::Rgb)
        "#,
    },
    DslShowcaseEntry {
        category: "Color Space Cmp",
        title: "fade cyan HSV",
        dsl: r#"
            fx::fade_to_fg(Color::Cyan, (3000, SineOut))
                .with_color_space(ColorSpace::Hsv)
        "#,
    },
    DslShowcaseEntry {
        category: "Color Space Cmp",
        title: "fade cyan HSL",
        dsl: r#"
            fx::fade_to_fg(Color::Cyan, (3000, SineOut))
                .with_color_space(ColorSpace::Hsl)
        "#,
    },
    DslShowcaseEntry {
        category: "Color Space Cmp",
        title: "fade cyan RGB",
        dsl: r#"
            fx::fade_to_fg(Color::Cyan, (3000, SineOut))
                .with_color_space(ColorSpace::Rgb)
        "#,
    },
    DslShowcaseEntry {
        category: "Color Space Cmp",
        title: "fade gold HSV",
        dsl: r#"
            fx::fade_to_fg(Color::Rgb(207, 181, 59), (3000, CubicOut))
                .with_color_space(ColorSpace::Hsv)
        "#,
    },
    DslShowcaseEntry {
        category: "Color Space Cmp",
        title: "fade gold HSL",
        dsl: r#"
            fx::fade_to_fg(Color::Rgb(207, 181, 59), (3000, CubicOut))
                .with_color_space(ColorSpace::Hsl)
        "#,
    },
    DslShowcaseEntry {
        category: "Color Space Cmp",
        title: "fade gold RGB",
        dsl: r#"
            fx::fade_to_fg(Color::Rgb(207, 181, 59), (3000, CubicOut))
                .with_color_space(ColorSpace::Rgb)
        "#,
    },
    // ── Pattern + Color Effect Combos ────────────────────────────────────
    DslShowcaseEntry {
        category: "Pattern+Color",
        title: "spiral dissolve + gold fade",
        dsl: r#"
            fx::parallel(&[
                fx::dissolve((4000, Linear))
                    .with_pattern(SpiralPattern::center().with_arms(4)),
                fx::fade_to_fg(Color::Rgb(207, 181, 59), (4000, QuadOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Pattern+Color",
        title: "diamond coalesce + copper paint",
        dsl: r#"
            fx::parallel(&[
                fx::coalesce((4000, SineOut))
                    .with_pattern(DiamondPattern::center()),
                fx::paint_fg(Color::Rgb(184, 115, 51), (4000, CubicOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Pattern+Color",
        title: "radial dissolve + teal paint",
        dsl: r#"
            fx::parallel(&[
                fx::dissolve((4000, QuadOut))
                    .with_pattern(RadialPattern::center()),
                fx::paint_fg(Color::Rgb(0, 180, 180), (4000, SineOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Pattern+Color",
        title: "sweep dissolve + crimson fade",
        dsl: r#"
            fx::parallel(&[
                fx::dissolve((4000, CubicOut))
                    .with_pattern(SweepPattern::left_to_right(5)),
                fx::fade_to_fg(Color::Rgb(220, 20, 60), (4000, QuadOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Pattern+Color",
        title: "checker dissolve + violet paint",
        dsl: r#"
            fx::parallel(&[
                fx::dissolve((4000, SineOut))
                    .with_pattern(CheckerboardPattern::default()),
                fx::paint_fg(Color::Rgb(138, 43, 226), (4000, CubicOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Pattern+Color",
        title: "diagonal dissolve + lime fade",
        dsl: r#"
            fx::parallel(&[
                fx::dissolve((4000, QuadOut))
                    .with_pattern(DiagonalPattern::top_left_to_bottom_right()),
                fx::fade_to_fg(Color::Rgb(50, 205, 50), (4000, SineOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Pattern+Color",
        title: "wave coalesce + orange paint",
        dsl: r#"
            fx::parallel(&[
                fx::coalesce((5000, Linear))
                    .with_pattern(WavePattern::new(
                        WaveLayer::new(Oscillator::sin(2.0, 1.0, 0.5))
                    )),
                fx::paint_fg(Color::Rgb(255, 140, 0), (5000, QuadOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Pattern+Color",
        title: "inverted radial + blue fade",
        dsl: r#"
            fx::parallel(&[
                fx::dissolve((4000, CubicOut))
                    .with_pattern(InvertedPattern::new(RadialPattern::center())),
                fx::fade_to_fg(Color::Blue, (4000, SineOut))
            ])
        "#,
    },
    // ── Sweep Speed Variations ───────────────────────────────────────────
    DslShowcaseEntry {
        category: "Sweep Speed",
        title: "sweep_in fast L→R",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sweep_in(Motion::LeftToRight, 10, 3, bg, (1500, QuadOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Sweep Speed",
        title: "sweep_in slow L→R",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sweep_in(Motion::LeftToRight, 10, 3, bg, (5000, SineOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Sweep Speed",
        title: "sweep_in narrow L→R",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sweep_in(Motion::LeftToRight, 4, 1, bg, (2500, CubicOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Sweep Speed",
        title: "sweep_in ultra-wide L→R",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sweep_in(Motion::LeftToRight, 30, 8, bg, (4500, SineOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Sweep Speed",
        title: "sweep_in bounce L→R",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sweep_in(Motion::LeftToRight, 10, 3, bg, (3000, BounceOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Sweep Speed",
        title: "sweep_in elastic R→L",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::sweep_in(Motion::RightToLeft, 10, 3, bg, (3500, ElasticOut))
        "#,
    },
    // ── Slide Speed Variations ───────────────────────────────────────────
    DslShowcaseEntry {
        category: "Slide Speed",
        title: "slide_in fast L→R",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::slide_in(Motion::LeftToRight, 8, 3, bg, (1500, QuadOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Slide Speed",
        title: "slide_in slow R→L",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::slide_in(Motion::RightToLeft, 8, 3, bg, (5000, SineOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Slide Speed",
        title: "slide_in narrow U→D",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::slide_in(Motion::UpToDown, 3, 1, bg, (2500, CubicOut))
        "#,
    },
    DslShowcaseEntry {
        category: "Slide Speed",
        title: "slide_in bounce D→U",
        dsl: r#"
            let bg = Color::Rgb(8, 9, 14);
            fx::slide_in(Motion::DownToUp, 8, 3, bg, (3000, BounceOut))
        "#,
    },
    // ── Dissolve Speed Variations ────────────────────────────────────────
    DslShowcaseEntry {
        category: "Dissolve Speed",
        title: "dissolve ultra-fast 500ms",
        dsl: "fx::dissolve(500)",
    },
    DslShowcaseEntry {
        category: "Dissolve Speed",
        title: "dissolve fast 1000ms",
        dsl: "fx::dissolve(1000)",
    },
    DslShowcaseEntry {
        category: "Dissolve Speed",
        title: "dissolve medium 2000ms",
        dsl: "fx::dissolve(2000)",
    },
    DslShowcaseEntry {
        category: "Dissolve Speed",
        title: "dissolve slow 4000ms",
        dsl: "fx::dissolve(4000)",
    },
    DslShowcaseEntry {
        category: "Dissolve Speed",
        title: "dissolve very slow 6000ms",
        dsl: "fx::dissolve((6000, Linear))",
    },
    DslShowcaseEntry {
        category: "Dissolve Speed",
        title: "coalesce ultra-fast 500ms",
        dsl: "fx::coalesce(500)",
    },
    DslShowcaseEntry {
        category: "Dissolve Speed",
        title: "coalesce fast 1000ms",
        dsl: "fx::coalesce(1000)",
    },
    DslShowcaseEntry {
        category: "Dissolve Speed",
        title: "coalesce slow 4000ms",
        dsl: "fx::coalesce((4000, SineOut))",
    },
    // ── Dissolve-Coalesce Cycle Combos ───────────────────────────────────
    DslShowcaseEntry {
        category: "Dissolve Cycles",
        title: "dissolve → coalesce fast",
        dsl: r#"
            fx::sequence(&[
                fx::dissolve(800),
                fx::coalesce(800)
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Dissolve Cycles",
        title: "dissolve → coalesce slow",
        dsl: r#"
            fx::sequence(&[
                fx::dissolve((2500, SineOut)),
                fx::coalesce((2500, CubicOut))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Dissolve Cycles",
        title: "dissolve radial → coalesce radial",
        dsl: r#"
            fx::sequence(&[
                fx::dissolve((2000, QuadOut))
                    .with_pattern(RadialPattern::center()),
                fx::coalesce((2000, SineOut))
                    .with_pattern(RadialPattern::center())
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Dissolve Cycles",
        title: "dissolve diamond → coalesce spiral",
        dsl: r#"
            fx::sequence(&[
                fx::dissolve((2000, CubicOut))
                    .with_pattern(DiamondPattern::center()),
                fx::coalesce((2000, QuadOut))
                    .with_pattern(SpiralPattern::center().with_arms(4))
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Dissolve Cycles",
        title: "dissolve spiral → coalesce checker",
        dsl: r#"
            fx::sequence(&[
                fx::dissolve((2000, SineOut))
                    .with_pattern(SpiralPattern::center().with_arms(3)),
                fx::coalesce((2000, CubicOut))
                    .with_pattern(CheckerboardPattern::default())
            ])
        "#,
    },
    DslShowcaseEntry {
        category: "Dissolve Cycles",
        title: "dissolve sweep → coalesce sweep",
        dsl: r#"
            fx::sequence(&[
                fx::dissolve((2000, QuadOut))
                    .with_pattern(SweepPattern::left_to_right(5)),
                fx::coalesce((2000, SineOut))
                    .with_pattern(SweepPattern::right_to_left(5))
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
        "Color::Rgb(207, 181, 59)",  // gold
        "Color::Rgb(184, 115, 51)",  // copper
        "Color::Rgb(192, 192, 192)", // silver
        "Color::Rgb(0, 180, 180)",   // teal
        "Color::Rgb(255, 105, 180)", // hot pink
        "Color::Rgb(100, 149, 237)", // cornflower
        "Color::Rgb(255, 140, 0)",   // dark orange
        "Color::Rgb(138, 43, 226)",  // blue violet
        "Color::Rgb(50, 205, 50)",   // lime green
        "Color::Rgb(220, 20, 60)",   // crimson
        "Color::Red",
        "Color::Blue",
        "Color::Green",
        "Color::Cyan",
        "Color::Magenta",
        "Color::Yellow",
    ];

    const INTERPS: &[&str] = &[
        "Linear",
        "QuadOut",
        "QuadIn",
        "CubicOut",
        "CubicIn",
        "CubicInOut",
        "SineOut",
        "SineIn",
        "BounceOut",
        "ExpoOut",
        "ElasticOut",
        "QuadInOut",
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
                "SweepPattern::left_to_right(5)".to_string(),
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
                format!("fx::fade_to_fg({}, ({}, {}))", color_a, duration, interp),
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
                format!("fx::paint_fg({}, ({}, {}))", color_a, duration, interp),
            )
        }
        7 => {
            // Evolve
            (
                "Procedural: Evolution".to_string(),
                format!("evolve #{}", index + 1),
                format!("fx::evolve({}, ({}, {}))", evolve, duration, interp),
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

/// Compile a DSL expression string into a looping Effect.
/// Tries multiple wrapping strategies to ensure all effects get infinite repeat.
/// A pause is inserted between repetitions so the effect name remains readable.
fn compile_dsl_effect(dsl_src: &str) -> Option<Effect> {
    let dsl = EffectDsl::new();
    let trimmed = dsl_src.trim();

    // Strategy 1: wrap in repeating(sequence(ping_pong(...), sleep)) via DSL
    let wrapped_pp = format!(
        "fx::repeating(fx::sequence(&[fx::ping_pong({}), fx::sleep(1500)]))",
        trimmed
    );
    if let Ok(effect) = dsl.compiler().compile(&wrapped_pp) {
        return Some(effect);
    }

    // Strategy 2: wrap in repeating(sequence(..., sleep)) via DSL (for effects that don't support ping_pong)
    let wrapped_rep = format!(
        "fx::repeating(fx::sequence(&[{}, fx::sleep(1500)]))",
        trimmed
    );
    if let Ok(effect) = dsl.compiler().compile(&wrapped_rep) {
        return Some(effect);
    }

    // Strategy 3: compile raw DSL (handles let bindings), wrap with Rust code
    if let Ok(effect) = dsl.compiler().compile(trimmed) {
        return Some(fx::repeating(fx::sequence(&[
            fx::ping_pong(effect),
            fx::sleep(1500),
        ])));
    }

    None
}

const BLOG_ENTRIES: &[(&str, &str, &str)] = &[
    (
        "Welcome to gold.silver.copper",
        "2025-01-15",
        include_str!("../blog/01-welcome.md"),
    ),
    (
        "Building Grift: A Minimalistic Lisp",
        "2025-02-01",
        include_str!("../blog/02-building-grift.md"),
    ),
    (
        "Vau Calculus Explained",
        "2025-03-10",
        include_str!("../blog/03-vau-calculus.md"),
    ),
    (
        "Terminal UIs in the Browser",
        "2025-04-20",
        include_str!("../blog/04-terminal-uis-browser.md"),
    ),
    (
        "Unified Layout Design",
        "2025-05-15",
        include_str!("../blog/05-unified-layout.md"),
    ),
    (
        "WebAssembly Performance",
        "2025-06-01",
        include_str!("../blog/06-wasm-performance.md"),
    ),
    (
        "TachyonFX: Shader Effects for TUIs",
        "2025-07-10",
        include_str!("../blog/07-tachyonfx.md"),
    ),
    (
        "Arena Allocation in Grift",
        "2025-08-05",
        include_str!("../blog/08-arena-allocation.md"),
    ),
];

const DOC_ENTRIES: &[(&str, &str)] = &[
    ("Grift Basics", include_str!("../docs/01-basics.md")),
    (
        "Special Forms & Definitions",
        include_str!("../docs/02-forms.md"),
    ),
    ("Advanced Features", include_str!("../docs/03-advanced.md")),
    (
        "Environments & Evaluation",
        include_str!("../docs/04-environments.md"),
    ),
    (
        "Error Handling & Debugging",
        include_str!("../docs/05-errors.md"),
    ),
];

#[derive(Clone, Copy, PartialEq)]
enum FocusMode {
    Outer,   // Arrow keys move across tabs; content scrolls passively
    Focused, // Active tab captures input; Escape returns to Outer
}

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Home,
    Repl,
    Docs,
    Blog,
    About,
    Effects,
}

impl Page {
    const ALL: [Page; 6] = [
        Page::Home,
        Page::Repl,
        Page::Docs,
        Page::Blog,
        Page::About,
        Page::Effects,
    ];

    fn title(self) -> &'static str {
        match self {
            Page::Home => "Home",
            Page::Repl => "REPL",
            Page::Docs => "Docs",
            Page::Blog => "Blog",
            Page::About => "About",
            Page::Effects => "Effects",
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
}

/// Convert a markdown string into styled ratatui Lines using md-tui parser.
fn md_to_lines(content: &str, width: u16) -> Vec<Line<'static>> {
    let mut root = md_tui::parser::parse_markdown(None, content, width);
    root.transform(width);
    let mut lines: Vec<Line<'static>> = Vec::new();

    for component in root.children() {
        match component {
            Component::TextComponent(tc) => {
                let kind = tc.kind();
                match kind {
                    TextNode::Heading => {
                        let heading_level = tc
                            .meta_info()
                            .iter()
                            .find_map(|w| match w.kind() {
                                WordType::MetaInfo(
                                    md_tui::nodes::word::MetaData::HeadingLevel(l),
                                ) => Some(l),
                                _ => None,
                            })
                            .unwrap_or(1);
                        let spans: Vec<Span<'static>> = tc
                            .content()
                            .iter()
                            .flatten()
                            .map(|w| md_word_to_span(w, heading_level))
                            .collect();
                        if !spans.is_empty() {
                            lines.push(
                                Line::from(spans)
                                    .style(Style::default().fg(Color::Rgb(220, 225, 235)).bold()),
                            );
                        }
                    }
                    TextNode::Paragraph | TextNode::Quote | TextNode::Task => {
                        for word_line in tc.content() {
                            let spans: Vec<Span<'static>> =
                                word_line.iter().map(|w| md_word_to_span(w, 0)).collect();
                            lines.push(Line::from(spans));
                        }
                    }
                    TextNode::List => {
                        for word_line in tc.content() {
                            let spans: Vec<Span<'static>> =
                                word_line.iter().map(|w| md_word_to_span(w, 0)).collect();
                            lines.push(Line::from(spans));
                        }
                    }
                    TextNode::CodeBlock => {
                        for word_line in tc.content() {
                            let spans: Vec<Span<'static>> =
                                word_line.iter().map(|w| md_word_to_span(w, 0)).collect();
                            lines.push(Line::from(spans));
                        }
                    }
                    TextNode::LineBreak => {
                        lines.push(Line::from(""));
                    }
                    TextNode::HorizontalSeparator => {
                        lines.push(Line::styled(
                            "─".repeat(width.saturating_sub(4) as usize),
                            Style::default().fg(Color::Rgb(55, 60, 70)),
                        ));
                    }
                    _ => {
                        // Fallback for other types
                        for word_line in tc.content() {
                            let spans: Vec<Span<'static>> =
                                word_line.iter().map(|w| md_word_to_span(w, 0)).collect();
                            lines.push(Line::from(spans));
                        }
                    }
                }
            }
            Component::Image(_) => {
                // Images not supported in WASM, skip
            }
        }
    }
    lines
}

/// Convert a single md-tui Word into a styled ratatui Span.
fn md_word_to_span(word: &md_tui::nodes::word::Word, heading_level: u8) -> Span<'static> {
    let content = word.content().to_string();
    match word.kind() {
        WordType::Bold => Span::styled(
            content,
            Style::default().fg(Color::Rgb(220, 225, 235)).bold(),
        ),
        WordType::Italic => Span::styled(
            content,
            Style::default().fg(Color::Rgb(184, 115, 51)).italic(),
        ),
        WordType::BoldItalic => Span::styled(
            content,
            Style::default()
                .fg(Color::Rgb(220, 225, 235))
                .bold()
                .italic(),
        ),
        WordType::Code => Span::styled(
            content,
            Style::default()
                .fg(Color::Rgb(207, 181, 59))
                .bg(Color::Rgb(30, 32, 40)),
        ),
        WordType::CodeBlock(color) => Span::styled(content, Style::default().fg(color)),
        WordType::Link | WordType::FootnoteInline => {
            Span::styled(content, Style::default().fg(Color::Rgb(100, 149, 237)))
        }
        WordType::Strikethrough => Span::styled(
            content,
            Style::default()
                .fg(Color::Rgb(100, 105, 115))
                .add_modifier(Modifier::CROSSED_OUT),
        ),
        WordType::ListMarker => {
            Span::styled(content, Style::default().fg(Color::Rgb(184, 115, 51)))
        }
        WordType::Normal | WordType::White => {
            if heading_level > 0 {
                Span::styled(
                    content,
                    Style::default().fg(Color::Rgb(220, 225, 235)).bold(),
                )
            } else {
                Span::styled(content, Style::default().fg(Color::Rgb(170, 175, 185)))
            }
        }
        _ => Span::styled(content, Style::default().fg(Color::Rgb(170, 175, 185))),
    }
}

struct App {
    page: Page,
    focus_mode: FocusMode,
    // REPL state
    repl_input: String,
    repl_cursor: usize,
    repl_history: Vec<(String, String)>,
    repl_scroll: usize,
    lisp: Box<Lisp<2000>>,
    // Docs state
    doc_index: usize,
    doc_viewing_section: bool,
    doc_scroll: usize,
    doc_item_areas: Vec<Rect>,
    doc_back_area: Rect,
    doc_list_area: Rect,
    doc_content_area: Rect,
    doc_nav_effect: Option<Effect>,
    prev_doc_index: usize,
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
    tab_page_indices: Vec<usize>,
    center_tab_idx: usize,
    carousel_effect: Option<Effect>,
    link_areas: Vec<Rect>,
    blog_item_areas: Vec<Rect>,
    blog_back_area: Rect,
    // Zone detection areas
    content_area: Rect,
    blog_list_area: Rect,
    blog_content_area: Rect,
    // Blog scroll
    blog_scroll: usize,
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
    // Navbar breathing effect tick (separate from bg_tick for independent rate)
    navbar_breath_tick: f64,
    // Virtual keyboard state (TVK)
    keyboard_shifted: bool,
    keyboard_pressed_ticks: Vec<(VirtualKey, u8)>,
    keyboard_button_areas: Vec<(Rect, String)>,
    keyboard_layout: tvk::layout::Layout,
    keyboard_env: tvk::env::Env,
    // Blog navigation effect
    blog_nav_effect: Option<Effect>,
    // Keyboard glow effect
    keyboard_glow_effect: Option<Effect>,
    // Track previous blog index for targeted nav effects
    prev_blog_index: usize,
}

impl App {
    fn new() -> Self {
        let lisp: Box<Lisp<2000>> = Box::new(Lisp::new());
        Self {
            page: Page::Home,
            focus_mode: FocusMode::Outer,
            repl_input: String::new(),
            repl_cursor: 0,
            repl_history: Vec::new(),
            repl_scroll: 0,
            lisp,
            doc_index: 0,
            doc_viewing_section: false,
            doc_scroll: 0,
            doc_item_areas: Vec::new(),
            doc_back_area: Rect::default(),
            doc_list_area: Rect::default(),
            doc_content_area: Rect::default(),
            doc_nav_effect: None,
            prev_doc_index: 0,
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
            tab_page_indices: Vec::new(),
            center_tab_idx: 0,
            carousel_effect: None,
            link_areas: Vec::new(),
            blog_item_areas: Vec::new(),
            blog_back_area: Rect::default(),
            content_area: Rect::default(),
            blog_list_area: Rect::default(),
            blog_content_area: Rect::default(),
            btn_effects: Vec::new(),
            tab_glow_effect: None,
            tab_hover_effects: Vec::new(),
            last_hovered_tab: None,
            link_hover_effects: Vec::new(),
            last_hovered_link: None,
            dsl_effects_scroll: 0,
            dsl_effects_cache: Vec::new(),
            frame_elapsed: Duration::from_millis(0),
            navbar_breath_tick: 0.0,
            keyboard_shifted: false,
            keyboard_pressed_ticks: Vec::new(),
            keyboard_button_areas: Vec::new(),
            keyboard_layout: lisp_keyboard_layout(),
            keyboard_env: {
                let mut env = tvk::env::Env::new();
                env.insert("border_color", tvk::env::Value::RGB(100, 105, 120));
                env.insert("highlight", tvk::env::Value::RGB(180, 185, 200));
                env
            },
            blog_nav_effect: None,
            keyboard_glow_effect: None,
            prev_blog_index: 0,
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
            Page::Docs => {
                let dsl = EffectDsl::new();
                match dsl.compiler().compile(
                    "fx::dissolve((300, QuadOut)).with_pattern(CheckerboardPattern::default())",
                ) {
                    Ok(effect) => effect,
                    Err(_) => fx::dissolve(EffectTimer::from_ms(300, Interpolation::QuadOut)),
                }
            }
            Page::Blog => fx::coalesce(EffectTimer::from_ms(400, Interpolation::SineOut)),
            Page::About => fx::sweep_in(
                Motion::UpToDown,
                8,
                2,
                dark,
                EffectTimer::from_ms(500, Interpolation::QuadOut),
            ),
            Page::Effects => fx::coalesce(EffectTimer::from_ms(500, Interpolation::SineOut)),
        };
        self.transition_effect = Some(effect);
    }

    fn reset_page_interaction_state(&mut self) {
        self.link_areas.clear();
        self.blog_item_areas.clear();
        self.blog_back_area = Rect::default();
        self.scroll_up_area = Rect::default();
        self.scroll_down_area = Rect::default();
        self.blog_list_area = Rect::default();
        self.blog_content_area = Rect::default();
    }

    fn carousel_transition_effect() -> Effect {
        let dark = Color::Rgb(8, 9, 14);
        fx::parallel(&[
            fx::slide_in(
                Motion::LeftToRight,
                4,
                2,
                dark,
                EffectTimer::from_ms(350, Interpolation::QuadOut),
            ),
            fx::sweep_in(
                Motion::LeftToRight,
                6,
                2,
                dark,
                EffectTimer::from_ms(300, Interpolation::SineOut),
            ),
            fx::fade_from(
                dark,
                dark,
                EffectTimer::from_ms(300, Interpolation::CubicOut),
            ),
        ])
    }

    fn switch_page(&mut self, page: Page) {
        if self.page != page {
            self.page = page;
            self.focus_mode = FocusMode::Outer;
            self.tab_glow_effect = None;
            self.blog_scroll = 0;
            self.blog_viewing_post = false;
            self.reset_page_interaction_state();
            self.trigger_transition();
            self.carousel_effect = Some(Self::carousel_transition_effect());
        }
    }

    fn switch_to_prev_tab(&mut self) {
        let idx = self.page.index();
        if idx > 0 {
            self.switch_page(Page::ALL[idx - 1]);
        } else {
            // Wrap around to last tab
            self.switch_page(Page::ALL[Page::ALL.len() - 1]);
        }
    }

    fn switch_to_next_tab(&mut self) {
        let idx = self.page.index();
        if idx + 1 < Page::ALL.len() {
            self.switch_page(Page::ALL[idx + 1]);
        } else {
            // Wrap around to first tab
            self.switch_page(Page::ALL[0]);
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
        let shift = fx::hsl_shift_fg([25.0, 12.0, 18.0], (500, Interpolation::SineOut));
        self.btn_effects.push((area, shift));
    }

    fn is_hovered(&self, area: Rect) -> bool {
        self.hover_col >= area.x
            && self.hover_col < area.right()
            && self.hover_row >= area.y
            && self.hover_row < area.bottom()
    }

    fn render_scroll_page(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        lines: Vec<Line<'static>>,
        scroll: &mut usize,
        hint: &str,
        outer_title: Option<Line<'static>>,
        inner_title: Option<Line<'static>>,
    ) {
        self.render_scrollable_content(frame, area, lines, scroll, outer_title, inner_title, hint);
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        match self.focus_mode {
            FocusMode::Outer => {
                // Outer mode: arrows move across tabs, Up/Down passively scroll content
                match key.code {
                    KeyCode::Left => self.switch_to_prev_tab(),
                    KeyCode::Right => self.switch_to_next_tab(),
                    KeyCode::Up | KeyCode::Down => {
                        // Passive scroll of current tab content
                        self.handle_passive_scroll(key);
                    }
                    KeyCode::Enter => {
                        // Enter focused mode for the current tab
                        self.focus_mode = FocusMode::Focused;
                    }
                    _ => {}
                }
            }
            FocusMode::Focused => {
                // Flash pressed key on virtual keyboard
                if self.page == Page::Repl {
                    if let Some(vk) = keycode_to_virtual_key(&key.code) {
                        // Remove any existing entry for this key and add fresh
                        self.keyboard_pressed_ticks.retain(|(k, _)| *k != vk);
                        self.keyboard_pressed_ticks.push((vk, 8));
                    }
                }
                // Check for Escape first — always returns to outer mode
                if key.code == KeyCode::Esc {
                    // For Blog post view, first exit to title list (stay focused)
                    if self.page == Page::Blog && self.blog_viewing_post {
                        self.blog_exit_post();
                        return;
                    }
                    // For Docs section view, first exit to section list (stay focused)
                    if self.page == Page::Docs && self.doc_viewing_section {
                        self.doc_exit_section();
                        return;
                    }
                    self.focus_mode = FocusMode::Outer;
                    self.keyboard_shifted = false;
                    self.keyboard_pressed_ticks.clear();
                    return;
                }
                // Delegate to page-specific focused handler
                match self.page {
                    Page::Repl => self.handle_repl_event(key),
                    Page::Blog => self.handle_blog_event(key),
                    Page::Docs => self.handle_docs_event(key),
                    Page::Home => self.handle_scroll_event_focused(key, ScrollTarget::Home),
                    Page::About => self.handle_scroll_event_focused(key, ScrollTarget::About),
                    Page::Effects => self.handle_effects_event_focused(key),
                }
            }
        }
    }

    /// Passive scroll in Outer mode — only Up/Down scroll content, no interaction
    fn handle_passive_scroll(&mut self, key: KeyEvent) {
        let step = 2;
        match self.page {
            Page::Repl => match key.code {
                KeyCode::Up => self.repl_scroll = self.repl_scroll.saturating_sub(1),
                KeyCode::Down => self.repl_scroll += 1,
                _ => {}
            },
            Page::Docs => match key.code {
                KeyCode::Up => self.doc_scroll = self.doc_scroll.saturating_sub(step),
                KeyCode::Down => self.doc_scroll += step,
                _ => {}
            },
            Page::Blog => match key.code {
                KeyCode::Up => self.blog_scroll = self.blog_scroll.saturating_sub(1),
                KeyCode::Down => self.blog_scroll += 1,
                _ => {}
            },
            Page::Home => match key.code {
                KeyCode::Up => self.home_scroll = self.home_scroll.saturating_sub(step),
                KeyCode::Down => self.home_scroll += step,
                _ => {}
            },
            Page::About => match key.code {
                KeyCode::Up => self.about_scroll = self.about_scroll.saturating_sub(step),
                KeyCode::Down => self.about_scroll += step,
                _ => {}
            },
            Page::Effects => match key.code {
                KeyCode::Up => {
                    self.dsl_effects_scroll = self.dsl_effects_scroll.saturating_sub(step)
                }
                KeyCode::Down => self.dsl_effects_scroll += step,
                _ => {}
            },
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
                        if let Some(&page_idx) = self.tab_page_indices.get(i) {
                            self.trigger_btn_effect(*tab_rect);
                            self.switch_page(Page::ALL[page_idx]);
                            // Clicking a tab returns to outer mode
                            self.focus_mode = FocusMode::Outer;
                            return;
                        }
                    }
                }
                // Clicked in tab bar but not on a tab — return to outer mode
                if self.focus_mode == FocusMode::Focused {
                    self.focus_mode = FocusMode::Outer;
                }
                return;
            }

            // Click in content area — enter focused mode if in outer mode
            if self.focus_mode == FocusMode::Outer
                && col >= self.content_area.x
                && col < self.content_area.right()
                && row >= self.content_area.y
                && row < self.content_area.bottom()
            {
                self.focus_mode = FocusMode::Focused;
            }

            // Click outside content area while focused — return to outer mode
            // On REPL page: only unfocus if tapping ABOVE the keyboard (tab bar),
            // not to the side or below it
            if self.focus_mode == FocusMode::Focused
                && (col < self.content_area.x
                    || col >= self.content_area.right()
                    || row < self.content_area.y
                    || row >= self.content_area.bottom())
            {
                if self.page == Page::Repl {
                    // Only unfocus when tapping above the content area (tab bar region)
                    if row < self.content_area.y {
                        self.focus_mode = FocusMode::Outer;
                    }
                } else {
                    self.focus_mode = FocusMode::Outer;
                }
                return;
            }

            // Check link clicks on About page
            if self.page == Page::About {
                for (i, area) in self.link_areas.iter().enumerate() {
                    if col >= area.x && col < area.right() && row >= area.y && row < area.bottom() {
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
                    self.trigger_btn_effect(self.blog_back_area);
                    self.blog_exit_post();
                    return;
                }

                for (i, area) in self.blog_item_areas.iter().enumerate() {
                    if col >= area.x
                        && col < area.right()
                        && row >= area.y
                        && row < area.bottom()
                        && i < BLOG_ENTRIES.len()
                    {
                        self.trigger_btn_effect(*area);
                        self.blog_open_post(i);
                        return;
                    }
                }
            }

            // Check docs item clicks
            if self.page == Page::Docs {
                // Check back button click
                if self.doc_back_area.width > 0
                    && col >= self.doc_back_area.x
                    && col < self.doc_back_area.right()
                    && row >= self.doc_back_area.y
                    && row < self.doc_back_area.bottom()
                {
                    self.trigger_btn_effect(self.doc_back_area);
                    self.doc_exit_section();
                    return;
                }

                for (i, area) in self.doc_item_areas.iter().enumerate() {
                    if col >= area.x
                        && col < area.right()
                        && row >= area.y
                        && row < area.bottom()
                        && i < DOC_ENTRIES.len()
                    {
                        self.trigger_btn_effect(*area);
                        self.doc_open_section(i);
                        return;
                    }
                }
            }

            // Virtual keyboard button clicks (REPL page, focused mode)
            if self.page == Page::Repl && self.focus_mode == FocusMode::Focused {
                let mut kbd_hit: Option<(Rect, String)> = None;
                for (btn_area, display_name) in &self.keyboard_button_areas {
                    if col >= btn_area.x
                        && col < btn_area.right()
                        && row >= btn_area.y
                        && row < btn_area.bottom()
                    {
                        kbd_hit = Some((*btn_area, display_name.clone()));
                        break;
                    }
                }
                if let Some((btn_area, display_name)) = kbd_hit {
                    self.handle_keyboard_tap(&display_name);
                    self.trigger_btn_effect(btn_area);
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
                self.handle_key_event(KeyEvent {
                    code: KeyCode::Up,
                    shift: false,
                    ctrl: false,
                    alt: false,
                });
                return;
            }
            if self.scroll_down_area.width > 0
                && col >= self.scroll_down_area.x
                && col < self.scroll_down_area.right()
                && row >= self.scroll_down_area.y
                && row < self.scroll_down_area.bottom()
            {
                self.trigger_btn_effect(self.scroll_down_area);
                self.handle_key_event(KeyEvent {
                    code: KeyCode::Down,
                    shift: false,
                    ctrl: false,
                    alt: false,
                });
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
                if key.ctrl {
                    // Emacs-style keybindings for REPL focused mode
                    match c {
                        'b' | 'B' => {
                            // Ctrl+B: move cursor backward
                            self.repl_cursor = self.repl_cursor.saturating_sub(1);
                        }
                        'f' | 'F' => {
                            // Ctrl+F: move cursor forward
                            let max = self.repl_input.chars().count();
                            if self.repl_cursor < max {
                                self.repl_cursor += 1;
                            }
                        }
                        'p' | 'P' => {
                            // Ctrl+P: scroll history up (previous)
                            self.repl_scroll = self.repl_scroll.saturating_sub(1);
                        }
                        'n' | 'N' => {
                            // Ctrl+N: scroll history down (next)
                            self.repl_scroll += 1;
                        }
                        _ => {}
                    }
                    return;
                }
                if key.alt {
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

    // ── Blog: Event Handling ──────────────────────────────────────────
    // Two modes: list (browsing titles) and post (reading content).
    // Focused list: Up/Down select, Enter/Right open.
    // Focused post: Up/Down scroll, Left/Esc back to list.
    // All selection goes through blog_select(); all opens through blog_open_post().

    fn handle_blog_event(&mut self, key: KeyEvent) {
        if self.blog_viewing_post {
            match key.code {
                KeyCode::Up => self.blog_scroll = self.blog_scroll.saturating_sub(1),
                KeyCode::Down => self.blog_scroll += 1,
                KeyCode::Left => self.blog_exit_post(),
                _ => {}
            }
        } else {
            let count = BLOG_ENTRIES.len();
            match key.code {
                KeyCode::Up if count > 0 => {
                    self.blog_select(self.blog_index.saturating_sub(1));
                }
                KeyCode::Down if count > 0 => {
                    self.blog_select((self.blog_index + 1).min(count - 1));
                }
                KeyCode::Enter | KeyCode::Right => {
                    self.blog_open_post(self.blog_index);
                }
                _ => {}
            }
        }
    }

    /// Move selection to `index`, triggering a nav effect on the affected items.
    fn blog_select(&mut self, index: usize) {
        if index >= BLOG_ENTRIES.len() || index == self.blog_index {
            return;
        }
        self.prev_blog_index = self.blog_index;
        self.blog_index = index;
        self.blog_scroll = 0;
        self.blog_nav_effect = Some(fx::fade_from(
            Color::Rgb(60, 65, 75),
            Color::Rgb(8, 9, 14),
            EffectTimer::from_ms(250, Interpolation::QuadOut),
        ));
    }

    /// Open a blog post by index. Selects it first if not already selected.
    fn blog_open_post(&mut self, index: usize) {
        if index >= BLOG_ENTRIES.len() {
            return;
        }
        if self.blog_index != index {
            self.blog_select(index);
        }
        self.blog_viewing_post = true;
        self.blog_scroll = 0;
        self.focus_mode = FocusMode::Focused;
        self.trigger_transition();
    }

    /// Return from post view to the title list.
    fn blog_exit_post(&mut self) {
        self.blog_viewing_post = false;
        self.blog_scroll = 0;
        self.trigger_transition();
    }

    // ── Docs: Event Handling ──────────────────────────────────────────
    // Two modes: list (browsing sections) and section (reading content).
    // Focused list: Up/Down select, Enter/Right open.
    // Focused section: Up/Down scroll, Left/Esc back to list.

    fn handle_docs_event(&mut self, key: KeyEvent) {
        if self.doc_viewing_section {
            match key.code {
                KeyCode::Up => self.doc_scroll = self.doc_scroll.saturating_sub(1),
                KeyCode::Down => self.doc_scroll += 1,
                KeyCode::Left => self.doc_exit_section(),
                _ => {}
            }
        } else {
            let count = DOC_ENTRIES.len();
            match key.code {
                KeyCode::Up if count > 0 => {
                    self.doc_select(self.doc_index.saturating_sub(1));
                }
                KeyCode::Down if count > 0 => {
                    self.doc_select((self.doc_index + 1).min(count - 1));
                }
                KeyCode::Enter | KeyCode::Right => {
                    self.doc_open_section(self.doc_index);
                }
                _ => {}
            }
        }
    }

    /// Move doc selection to `index`, triggering a nav effect.
    fn doc_select(&mut self, index: usize) {
        if index >= DOC_ENTRIES.len() || index == self.doc_index {
            return;
        }
        self.prev_doc_index = self.doc_index;
        self.doc_index = index;
        self.doc_scroll = 0;
        self.doc_nav_effect = Some(fx::fade_from(
            Color::Rgb(60, 65, 75),
            Color::Rgb(8, 9, 14),
            EffectTimer::from_ms(250, Interpolation::QuadOut),
        ));
    }

    /// Open a doc section by index.
    fn doc_open_section(&mut self, index: usize) {
        if index >= DOC_ENTRIES.len() {
            return;
        }
        if self.doc_index != index {
            self.doc_select(index);
        }
        self.doc_viewing_section = true;
        self.doc_scroll = 0;
        self.focus_mode = FocusMode::Focused;
        self.trigger_transition();
    }

    /// Return from section view to the section list.
    fn doc_exit_section(&mut self) {
        self.doc_viewing_section = false;
        self.doc_scroll = 0;
        self.trigger_transition();
    }

    fn handle_keyboard_tap(&mut self, display_name: &str) {
        match display_name {
            "⇧" => {
                self.keyboard_shifted = !self.keyboard_shifted;
            }
            "⌫" => {
                let key = KeyEvent {
                    code: KeyCode::Backspace,
                    ctrl: false,
                    alt: false,
                    shift: false,
                };
                self.handle_repl_event(key);
            }
            "ENTER" => {
                let key = KeyEvent {
                    code: KeyCode::Enter,
                    ctrl: false,
                    alt: false,
                    shift: false,
                };
                self.handle_repl_event(key);
            }
            " " => {
                let key = KeyEvent {
                    code: KeyCode::Char(' '),
                    ctrl: false,
                    alt: false,
                    shift: false,
                };
                self.handle_repl_event(key);
            }
            name => {
                // Regular character key — dispatch as Char
                if let Some(ch) = name.chars().next() {
                    let key = KeyEvent {
                        code: KeyCode::Char(ch),
                        ctrl: false,
                        alt: false,
                        shift: false,
                    };
                    self.handle_repl_event(key);
                    // Turn off shift after typing a character (sticky shift)
                    if self.keyboard_shifted {
                        self.keyboard_shifted = false;
                    }
                }
            }
        }
    }

    fn handle_scroll_event_focused(&mut self, key: KeyEvent, target: ScrollTarget) {
        let scroll = match target {
            ScrollTarget::Home => &mut self.home_scroll,
            ScrollTarget::About => &mut self.about_scroll,
        };
        let step = 2;
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

    fn handle_effects_event_focused(&mut self, key: KeyEvent) {
        let step = 2;
        match key.code {
            KeyCode::Up => {
                self.dsl_effects_scroll = self.dsl_effects_scroll.saturating_sub(step);
            }
            KeyCode::Down => {
                self.dsl_effects_scroll += step;
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
        // Advance navbar breathing tick (slow, calm rhythm)
        self.navbar_breath_tick += elapsed.as_millis() as f64 * 0.001;

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
        // Use 1-tile border on narrow screens (<80 tiles) for better usability
        let h_margin = if full_area.width < 80 {
            1
        } else {
            (full_area.width / MARGIN_DIVISOR).min(2)
        };
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

        // ── Navbar breathing effect ─────────────────────────────────────
        // Per-character darken_fg: center brighter, edges dimmer, with
        // procedural per-column offset for an organic breathing pulse.
        {
            let buf = frame.buffer_mut();
            let tab_row = tab_area.y + 1; // text row inside bordered tab area
            let tab_left = tab_area.x + 1;
            let tab_right = tab_area.right().saturating_sub(1);
            let tab_span = (tab_right as f64 - tab_left as f64).max(1.0);
            let center = tab_left as f64 + tab_span * 0.5;
            let t = self.navbar_breath_tick;

            for x in tab_left..tab_right {
                let pos = Position::new(x, tab_row);
                if let Some(cell) = buf.cell_mut(pos) {
                    // Normalized distance from center: 0.0 = center, 1.0 = edge
                    let dist = ((x as f64 - center).abs() / (tab_span * 0.5)).min(1.0);

                    // Per-character phase offset for organic wave feel
                    let phase_offset = (x as f64 - tab_left as f64) * 0.18;

                    // Slow breathing: sin wave with ~6s period per cycle
                    let breath = (t * 1.05 + phase_offset).sin() * 0.5 + 0.5;

                    // Base darkening: center gets light dimming, edges get heavy dimming
                    // Range: center ~0.15, edges ~0.75 (dim & tarnished)
                    let base_darken = 0.15 + dist * 0.60;

                    // Breathing modulates darkening by ±0.12
                    let darken = (base_darken + breath * 0.12).min(0.90);

                    // Apply darken_fg: reduce each RGB channel of fg
                    let keep = 1.0 - darken;
                    let (r, g, b) = match cell.fg {
                        Color::Rgb(r, g, b) => (r, g, b),
                        _ => (200, 200, 210),
                    };
                    cell.set_fg(Color::Rgb(
                        (r as f64 * keep) as u8,
                        (g as f64 * keep) as u8,
                        (b as f64 * keep) as u8,
                    ));
                }
            }
        }

        // Render fire glow effect on the selected tab
        if let Some(selected_tab_rect) = self.tab_rects.get(self.center_tab_idx).copied() {
            if self.tab_glow_effect.is_none() {
                // Subtle warm copper/gold hsl shift
                let fg_shift = [8.0, 10.0, 6.0];
                let timer = (1800, Interpolation::SineIn);
                let glow = fx::hsl_shift_fg(fg_shift, timer).with_filter(CellFilter::Text);
                self.tab_glow_effect = Some(fx::repeating(fx::ping_pong(glow)));
            }
            if let Some(ref mut effect) = self.tab_glow_effect {
                frame.render_effect(effect, selected_tab_rect, elapsed);
            }
        }

        // Tab hover translate effect — triggers when a new tab is hovered
        let current_hovered_tab = self
            .tab_rects
            .iter()
            .enumerate()
            .find(|(_, r)| self.is_hovered(**r))
            .map(|(i, _)| i);
        if current_hovered_tab != self.last_hovered_tab {
            if let Some(idx) = current_hovered_tab {
                if self.tab_rects.get(idx).is_some() {
                    let dissolve = fx::dissolve(EffectTimer::from_ms(400, Interpolation::QuadOut));
                    self.tab_hover_effects.push((idx, dissolve));
                    if let Some(tab_rect) = self.tab_rects.get(idx).copied() {
                        let shift =
                            fx::hsl_shift_fg([20.0, 10.0, 14.0], (450, Interpolation::SineOut));
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

        // Process carousel transition effect
        if let Some(ref mut effect) = self.carousel_effect {
            if effect.running() {
                frame.render_effect(effect, tab_area, elapsed);
            }
        }
        if self.carousel_effect.as_ref().is_some_and(|e| !e.running()) {
            self.carousel_effect = None;
        }

        match self.page {
            Page::Home => self.render_home(frame, content_area),
            Page::Repl => self.render_repl(frame, content_area),
            Page::Docs => self.render_docs(frame, content_area),
            Page::Blog => self.render_blog(frame, content_area),
            Page::About => self.render_about(frame, content_area),
            Page::Effects => self.render_effects(frame, content_area),
        }

        // Process blog navigation effect — apply only to the affected title areas
        if let Some(ref mut effect) = self.blog_nav_effect {
            if effect.running() {
                // Apply effect only to the new and previous blog title areas
                if let Some(new_area) = self.blog_item_areas.get(self.blog_index).copied() {
                    if new_area.width > 0 {
                        frame.render_effect(effect, new_area, elapsed);
                    }
                }
                if self.prev_blog_index != self.blog_index {
                    if let Some(old_area) = self.blog_item_areas.get(self.prev_blog_index).copied()
                    {
                        if old_area.width > 0 {
                            // Subtle inverse HSL shift to dim the old title (negative hue/sat/light)
                            let mut reverse = fx::hsl_shift_fg(
                                [-10.0, -5.0, -8.0],
                                (250, Interpolation::QuadOut),
                            );
                            frame.render_effect(&mut reverse, old_area, elapsed);
                        }
                    }
                }
            }
        }
        if self.blog_nav_effect.as_ref().is_some_and(|e| !e.running()) {
            self.blog_nav_effect = None;
        }

        // Focus indicator: subtle border highlight when in focused mode
        if self.focus_mode == FocusMode::Focused {
            let buf = frame.buffer_mut();
            let focus_fg = Color::Rgb(100, 110, 130);
            // Highlight the top and bottom border of content area
            for x in content_area.x..content_area.right() {
                for &y in &[content_area.y, content_area.bottom().saturating_sub(1)] {
                    let pos = Position::new(x, y);
                    if let Some(cell) = buf.cell_mut(pos) {
                        cell.set_fg(focus_fg);
                    }
                }
            }
            // Highlight the left and right border of content area
            for y in content_area.y..content_area.bottom() {
                for &x in &[content_area.x, content_area.right().saturating_sub(1)] {
                    let pos = Position::new(x, y);
                    if let Some(cell) = buf.cell_mut(pos) {
                        cell.set_fg(focus_fg);
                    }
                }
            }
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
            let current_hovered_link = self
                .link_areas
                .iter()
                .enumerate()
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
                        let shift_fx =
                            fx::hsl_shift_fg([15.0, 8.0, 10.0], (500, Interpolation::SineOut));
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

        let pad: u16 = 1;
        let inner_x = area.x + 1; // after left border
        let inner_width = area.width.saturating_sub(2);
        let tab_row = area.y + 1;
        let divider_width: u16 = 1;

        let num_pages = Page::ALL.len();
        let selected_idx = self.page.index();

        // Compute padded widths for each page
        let tab_widths: Vec<u16> = Page::ALL
            .iter()
            .map(|p| p.title().len() as u16 + pad * 2)
            .collect();

        // Calculate center position for selected tab
        let selected_width = tab_widths[selected_idx];
        let center_x = inner_x as i32 + (inner_width as i32 - selected_width as i32) / 2;

        // Collect carousel entries: (page_idx, x_position, width, is_center)
        let mut entries: Vec<(usize, i32, u16, bool)> = Vec::new();

        // Center entry
        entries.push((selected_idx, center_x, selected_width, true));

        // Fill rightward
        let right_edge = (inner_x + inner_width) as i32;
        let mut cursor = center_x + selected_width as i32 + divider_width as i32;
        let mut idx = (selected_idx + 1) % num_pages;
        while cursor < right_edge {
            let w = tab_widths[idx];
            entries.push((idx, cursor, w, false));
            cursor += w as i32 + divider_width as i32;
            idx = (idx + 1) % num_pages;
        }

        // Fill leftward
        idx = if selected_idx == 0 {
            num_pages - 1
        } else {
            selected_idx - 1
        };
        cursor = center_x - divider_width as i32;
        loop {
            let w = tab_widths[idx];
            let tab_start = cursor - w as i32;
            entries.push((idx, tab_start, w, false));
            if tab_start <= inner_x as i32 {
                break;
            }
            cursor = tab_start - divider_width as i32;
            idx = if idx == 0 { num_pages - 1 } else { idx - 1 };
        }

        // Sort entries by x position for consistent rendering
        entries.sort_by_key(|e| e.1);

        // Render the block border
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(55, 60, 70))
            .title(Line::from(" GRIFT.RS ").alignment(Alignment::Center))
            .title_style(Style::default().fg(Color::Rgb(207, 181, 59)).bold());
        frame.render_widget(block, area);

        // Build tab_rects, tab_page_indices, and render text directly into buffer
        let vis_left = inner_x;
        let vis_right = inner_x + inner_width;

        self.tab_rects.clear();
        self.tab_page_indices.clear();
        self.center_tab_idx = 0;

        let buf = frame.buffer_mut();

        for &(page_idx, x, width, is_center) in &entries {
            let raw_x = x;
            let raw_right = x + width as i32;
            let clipped_x = raw_x.max(vis_left as i32) as u16;
            let clipped_right = (raw_right.min(vis_right as i32) as u16).min(vis_right);

            if clipped_right > clipped_x {
                let rect_idx = self.tab_rects.len();
                self.tab_rects
                    .push(Rect::new(clipped_x, tab_row, clipped_right - clipped_x, 1));
                self.tab_page_indices.push(page_idx);

                if is_center {
                    self.center_tab_idx = rect_idx;
                }

                let style = self.tab_style(
                    Rect::new(clipped_x, tab_row, clipped_right - clipped_x, 1),
                    is_center,
                );

                // Write tab text into buffer, handling clipping
                let page = Page::ALL[page_idx];
                let padded_title = format!(" {} ", page.title());
                let chars: Vec<char> = padded_title.chars().collect();
                let char_offset = (clipped_x as i32 - raw_x).max(0) as usize;

                for (ci, &ch) in chars.iter().enumerate().skip(char_offset) {
                    let cx = raw_x + ci as i32;
                    if cx < vis_left as i32 {
                        continue;
                    }
                    if cx >= vis_right as i32 {
                        break;
                    }
                    let pos = Position::new(cx as u16, tab_row);
                    if let Some(cell) = buf.cell_mut(pos) {
                        cell.set_char(ch);
                        cell.set_style(style);
                    }
                }
            }

            // Render divider after this tab
            let div_x = raw_right;
            if div_x >= vis_left as i32 && div_x < vis_right as i32 {
                let pos = Position::new(div_x as u16, tab_row);
                if let Some(cell) = buf.cell_mut(pos) {
                    cell.set_char('│');
                    cell.set_style(Style::default().fg(Color::Rgb(100, 105, 115)));
                }
            }
        }
    }

    fn tab_style(&self, area: Rect, is_center: bool) -> Style {
        let hovered = self.is_hovered(area);
        let fg = if is_center {
            Color::Rgb(230, 232, 240)
        } else if hovered {
            Color::Rgb(255, 255, 255)
        } else {
            Color::Rgb(140, 145, 155)
        };

        if is_center {
            Style::default()
                .fg(fg)
                .bold()
                .add_modifier(Modifier::REVERSED)
        } else if hovered {
            Style::default().fg(fg).bold()
        } else {
            Style::default().fg(fg)
        }
    }

    fn render_home(&mut self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        push_section(
            &mut lines,
            "── Grift ──",
            DESCRIPTION,
            SectionStyle::new(GOLD, BODY_TEXT_COLOR),
        );
        push_section(
            &mut lines,
            "── Fexprs & Vau Calculus ──",
            VAU_INFO,
            SectionStyle::new(COPPER, BODY_TEXT_COLOR),
        );
        push_section(
            &mut lines,
            "── First-Class Everything ──",
            FIRST_CLASS_INFO,
            SectionStyle::new(SILVER, BODY_TEXT_COLOR),
        );
        push_section(
            &mut lines,
            "── Implementation ──",
            IMPL_INFO,
            SectionStyle::new(Color::Rgb(222, 165, 132), BODY_TEXT_COLOR),
        );
        push_section(
            &mut lines,
            "── Why Grift? ──",
            "Most Lisps distinguish between functions and macros at a fundamental level. Grift eliminates this distinction entirely through vau calculus. Every combiner is an operative that can choose whether to evaluate its arguments. This makes the language simpler, more uniform, and more powerful. If you can write a function, you can write a macro — they are the same thing.",
            SectionStyle::new(LINK_TEXT_COLOR, BODY_TEXT_COLOR),
        );
        push_section(
            &mut lines,
            "── This Site ──",
            "Everything you see is a Rust terminal UI compiled to WebAssembly and rendered to an HTML canvas via Ratzilla. TachyonFX provides the animated background, page transitions, and hover effects. There is no HTML layout, no CSS styling, and no JavaScript framework — just a Rust application drawing characters to a terminal grid. The same layout works on every device and screen size.",
            SectionStyle::new(GOLD, BODY_TEXT_COLOR),
        );
        push_section(
            &mut lines,
            "── Getting Started ──",
            "Try Grift right now — switch to the REPL tab and type (+ 1 2). Browse the Docs tab for the full language reference. Check the Effects tab to see TachyonFX visual effects in action. All tabs are accessible via touch, mouse, or keyboard.",
            SectionStyle::new(COPPER, BODY_TEXT_COLOR),
        );
        lines.push(Line::styled(
            "── Features at a Glance ──",
            Style::default().fg(LINK_TEXT_COLOR).bold(),
        ));
        push_blank_line(&mut lines);
        let features = "• Interactive REPL with full Grift interpreter\n• Animated background and page transitions\n• Clickable tabs, links, and buttons\n• Mobile-first touch gesture support\n• Momentum scrolling and swipe navigation\n• Zero JavaScript frameworks — pure Rust + WASM";
        push_styled_multiline(&mut lines, features, Style::default().fg(BODY_TEXT_COLOR));

        let home_hint = if self.focus_mode == FocusMode::Focused {
            "Esc: unfocus │ ↑↓: scroll │ ←→: tabs"
        } else {
            "Enter/tap: focus │ ←→: tabs │ ↑↓: scroll"
        };

        let mut scroll = self.home_scroll;
        self.render_scroll_page(frame, area, lines, &mut scroll, home_hint, None, None);
        self.home_scroll = scroll;
    }

    fn render_repl(&mut self, frame: &mut Frame, area: Rect) {
        // Determine keyboard height: 5 rows × 3 lines = 15 lines for keyboard
        let show_keyboard = self.focus_mode == FocusMode::Focused;
        let kbd_height = if show_keyboard { 15 } else { 0 };

        let (repl_area, kbd_area) = if show_keyboard {
            let [r, k] =
                Layout::vertical([Constraint::Min(6), Constraint::Length(kbd_height)]).areas(area);
            (r, Some(k))
        } else {
            (area, None)
        };

        let hint_text = if show_keyboard {
            "│ Esc: unfocus │ ←→: cursor │ type + ↵ │"
        } else {
            "│ Enter/tap: focus │ ←→: tabs │ ↑↓: scroll │"
        };

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(55, 60, 70))
            .title(" Grift REPL ".bold().fg(Color::Rgb(184, 115, 51)))
            .title_bottom(
                Line::from(hint_text)
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::Rgb(55, 60, 70))),
            );

        let inner = block.inner(repl_area);
        frame.render_widget(block, repl_area);

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
        self.render_blinking_cursor(
            frame,
            input_area.x + 1 + 3 + self.repl_cursor as u16,
            input_area.y + 1,
            input_area.right() - 1,
        );

        // History (newest first)
        let mut history_lines: Vec<Line> = Vec::new();
        for (input, output) in self.repl_history.iter().rev() {
            history_lines.push(Line::from(vec![
                Span::styled("Λ> ", Style::default().fg(Color::Rgb(184, 115, 51)).bold()),
                Span::styled(
                    input.as_str(),
                    Style::default().fg(Color::Rgb(200, 200, 210)),
                ),
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
                "  Try: (+ 1 2), (list 1 2 3), (define! x 42)".fg(Color::Rgb(100, 105, 115)),
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

        // Render virtual keyboard when focused
        if let Some(kbd_area) = kbd_area {
            // Decay keyboard press flash timers
            self.keyboard_pressed_ticks.retain_mut(|(_, ticks)| {
                *ticks = ticks.saturating_sub(1);
                *ticks > 0
            });

            // Build pressed keys set for rendering
            let mut pressed: HashSet<VirtualKey> = self
                .keyboard_pressed_ticks
                .iter()
                .map(|(k, _)| *k)
                .collect();
            if self.keyboard_shifted {
                pressed.insert(VirtualKey::ShiftLeft);
            }

            // Render keyboard and store button areas for click handling
            self.keyboard_button_areas = tvk::render::render_keyboard_inline(
                frame,
                kbd_area,
                &pressed,
                &self.keyboard_layout,
                &self.keyboard_env,
            );

            // Repeating passive silver glow effect on the keyboard area
            let elapsed = self.frame_elapsed;
            if self.keyboard_glow_effect.is_none() {
                // [hue_shift, saturation_shift, lightness_shift]: neutral hue, slight desaturation, gentle brightness pulse
                let glow = fx::hsl_shift_fg([0.0, -3.0, 8.0], (3000, Interpolation::SineIn));
                self.keyboard_glow_effect = Some(fx::repeating(fx::ping_pong(glow)));
            }
            if let Some(ref mut glow) = self.keyboard_glow_effect {
                frame.render_effect(glow, kbd_area, elapsed);
            }
        } else {
            self.keyboard_button_areas.clear();
        }
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

    // ── Docs: Rendering ──────────────────────────────────────────────
    // Two views dispatched from render_docs:
    //   render_doc_section — full section with back button and scrollable content
    //   render_docs_list   — scrollable list of section titles with selection highlight

    fn render_docs(&mut self, frame: &mut Frame, area: Rect) {
        self.doc_back_area = Rect::default();
        if self.doc_viewing_section {
            self.render_doc_section(frame, area);
        } else {
            self.render_docs_list(frame, area);
        }
    }

    fn render_doc_section(&mut self, frame: &mut Frame, area: Rect) {
        self.doc_list_area = Rect::default();
        self.doc_item_areas.clear();

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(55, 60, 70))
            .title(" Documentation ".bold().fg(Color::Rgb(200, 200, 210)));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let [back_bar, scroll_area, nav_bar] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .areas(inner);

        // Back button
        let back_style = if self.is_hovered(back_bar) {
            Style::default().fg(Color::Rgb(255, 255, 255)).bold()
        } else {
            Style::default().fg(Color::Rgb(184, 115, 51))
        };
        frame.render_widget(
            Paragraph::new("◄ Back to sections").style(back_style),
            back_bar,
        );
        self.doc_back_area = back_bar;

        // Section content — parse markdown via md-tui
        let (_title, content) = match DOC_ENTRIES.get(self.doc_index) {
            Some(entry) => *entry,
            None => return,
        };

        let content_width = scroll_area.width.saturating_sub(4);
        let lines = md_to_lines(content, content_width);

        let visible_height = scroll_area.height.saturating_sub(2) as usize;
        let content_width = scroll_area.width.saturating_sub(2) as usize;
        let total_wrapped = Self::wrapped_line_count(&lines, content_width);
        let max_scroll = total_wrapped.saturating_sub(visible_height);
        self.doc_scroll = self.doc_scroll.min(max_scroll);

        let section = Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .scroll((self.doc_scroll as u16, 0))
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Color::Rgb(40, 44, 52)),
            );
        frame.render_widget(section, scroll_area);
        self.doc_content_area = scroll_area;

        self.render_vertical_scrollbar(frame, scroll_area, self.doc_scroll, max_scroll);

        let hint = if self.focus_mode == FocusMode::Focused {
            "Esc/←: back │ ↑↓: scroll"
        } else {
            "Enter/tap: focus │ ←→: tabs"
        };
        self.render_scroll_arrows(frame, nav_bar, self.doc_scroll, max_scroll, hint);
    }

    fn render_docs_list(&mut self, frame: &mut Frame, area: Rect) {
        self.doc_list_area = area;
        self.doc_content_area = Rect::default();

        // Build list lines — one title per entry
        let mut lines: Vec<Line> = Vec::new();
        let mut entry_line_indices: Vec<usize> = Vec::new();

        for (i, (title, _)) in DOC_ENTRIES.iter().enumerate() {
            entry_line_indices.push(lines.len());

            let is_selected = self.doc_index == i;
            let is_hovered = self
                .doc_item_areas
                .get(i)
                .is_some_and(|r| self.is_hovered(*r));

            let active = is_selected || is_hovered;
            let (style, marker) = if active {
                (Style::default().fg(Color::Rgb(207, 181, 59)).bold(), "▸ ")
            } else {
                (Style::default().fg(Color::Rgb(200, 200, 210)), "  ")
            };

            lines.push(Line::from(format!("{marker}{title}")).style(style));
            lines.push(Line::from(""));
        }

        let hint = if self.focus_mode == FocusMode::Focused {
            "Esc: unfocus │ ↑↓: select │ Enter/→: read"
        } else {
            "Enter/tap: focus │ ←→: tabs │ tap section"
        };

        let mut scroll = self.doc_scroll;
        let scroll_area = self.render_scrollable_content(
            frame,
            area,
            lines,
            &mut scroll,
            Some(
                " Documentation "
                    .bold()
                    .fg(Color::Rgb(200, 200, 210))
                    .into(),
            ),
            Some(
                " Grift Language Reference — tap to read "
                    .bold()
                    .fg(Color::Rgb(184, 115, 51))
                    .into(),
            ),
            hint,
        );
        self.doc_scroll = scroll;

        // Compute click areas for each doc title relative to scroll position
        let content_inner = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(40, 44, 52))
            .inner(scroll_area);

        self.doc_item_areas.clear();
        for &line_idx in &entry_line_indices {
            if line_idx >= self.doc_scroll {
                let visible_row = (line_idx - self.doc_scroll) as u16;
                if visible_row + 1 <= content_inner.height {
                    self.doc_item_areas.push(Rect::new(
                        content_inner.x,
                        content_inner.y + visible_row,
                        content_inner.width,
                        1,
                    ));
                } else {
                    self.doc_item_areas.push(Rect::default());
                }
            } else {
                self.doc_item_areas.push(Rect::default());
            }
        }
    }

    // ── Blog: Rendering ──────────────────────────────────────────────
    // Two views dispatched from render_blog:
    //   render_blog_post  — full post with back button and scrollable content
    //   render_blog_list  — scrollable list of titles with selection highlight

    fn render_blog(&mut self, frame: &mut Frame, area: Rect) {
        self.blog_back_area = Rect::default();
        if self.blog_viewing_post {
            self.render_blog_post(frame, area);
        } else {
            self.render_blog_list(frame, area);
        }
    }

    fn render_blog_post(&mut self, frame: &mut Frame, area: Rect) {
        self.blog_list_area = Rect::default();
        self.blog_item_areas.clear();

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(55, 60, 70))
            .title(" Blog ".bold().fg(Color::Rgb(200, 200, 210)));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let [back_bar, scroll_area, nav_bar] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .areas(inner);

        // Back button
        let back_style = if self.is_hovered(back_bar) {
            Style::default().fg(Color::Rgb(255, 255, 255)).bold()
        } else {
            Style::default().fg(Color::Rgb(184, 115, 51))
        };
        frame.render_widget(
            Paragraph::new("◄ Back to posts").style(back_style),
            back_bar,
        );
        self.blog_back_area = back_bar;

        // Post content — parse markdown via md-tui
        let (_title, _date, content) = match BLOG_ENTRIES.get(self.blog_index) {
            Some(entry) => *entry,
            None => return,
        };

        let content_width = scroll_area.width.saturating_sub(4);
        let lines = md_to_lines(content, content_width);

        let visible_height = scroll_area.height.saturating_sub(2) as usize;
        let content_width = scroll_area.width.saturating_sub(2) as usize;
        let total_wrapped = Self::wrapped_line_count(&lines, content_width);
        let max_scroll = total_wrapped.saturating_sub(visible_height);
        self.blog_scroll = self.blog_scroll.min(max_scroll);

        let post = Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .scroll((self.blog_scroll as u16, 0))
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(Color::Rgb(40, 44, 52)),
            );
        frame.render_widget(post, scroll_area);
        self.blog_content_area = scroll_area;

        self.render_vertical_scrollbar(frame, scroll_area, self.blog_scroll, max_scroll);

        let hint = if self.focus_mode == FocusMode::Focused {
            "Esc/←: back │ ↑↓: scroll"
        } else {
            "Enter/tap: focus │ ←→: tabs"
        };
        self.render_scroll_arrows(frame, nav_bar, self.blog_scroll, max_scroll, hint);
    }

    fn render_blog_list(&mut self, frame: &mut Frame, area: Rect) {
        self.blog_list_area = area;
        self.blog_content_area = Rect::default();

        // Build list lines — one title + date + blank per entry
        let mut lines: Vec<Line> = Vec::new();
        let mut entry_line_indices: Vec<usize> = Vec::new();

        for (i, (title, date, _)) in BLOG_ENTRIES.iter().enumerate() {
            entry_line_indices.push(lines.len());

            let is_selected = self.blog_index == i;
            let is_hovered = self
                .blog_item_areas
                .get(i)
                .is_some_and(|r| self.is_hovered(*r));

            // Unified single selector: hover updates selection on mobile, highlight is always consistent
            let active = is_selected || is_hovered;
            let (style, marker) = if active {
                (Style::default().fg(Color::Rgb(207, 181, 59)).bold(), "▸ ")
            } else {
                (Style::default().fg(Color::Rgb(200, 200, 210)), "  ")
            };

            lines.push(Line::from(format!("{marker}{title}")).style(style));

            let date_style = if active {
                Style::default().fg(Color::Rgb(184, 115, 51))
            } else {
                Style::default().fg(Color::Rgb(75, 80, 90))
            };
            lines.push(Line::styled(format!("    {date}"), date_style));
            lines.push(Line::from(""));
        }

        let hint = if self.focus_mode == FocusMode::Focused {
            "Esc: unfocus │ ↑↓: select │ Enter/→: read"
        } else {
            "Enter/tap: focus │ ←→: tabs │ tap post"
        };

        let mut scroll = self.blog_scroll;
        let scroll_area = self.render_scrollable_content(
            frame,
            area,
            lines,
            &mut scroll,
            Some(" Blog ".bold().fg(Color::Rgb(200, 200, 210)).into()),
            Some(
                " Posts — tap to read "
                    .bold()
                    .fg(Color::Rgb(184, 115, 51))
                    .into(),
            ),
            hint,
        );
        self.blog_scroll = scroll;

        // Compute click areas for each blog title relative to scroll position
        let content_inner = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(40, 44, 52))
            .inner(scroll_area);

        self.blog_item_areas.clear();
        for &line_idx in &entry_line_indices {
            if line_idx >= self.blog_scroll {
                let visible_row = (line_idx - self.blog_scroll) as u16;
                if visible_row + 2 <= content_inner.height {
                    // Touch target covers both title and date rows for mobile usability
                    self.blog_item_areas.push(Rect::new(
                        content_inner.x,
                        content_inner.y + visible_row,
                        content_inner.width,
                        2,
                    ));
                } else if visible_row < content_inner.height {
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

        push_section(
            &mut lines,
            "── gold silver copper ──",
            "Software developer building open-source tools in Rust. Interested in programming language design, terminal user interfaces, WebAssembly, and making the web a stranger and more interesting place. Creator of Grift and this terminal-in-a-browser website.",
            SectionStyle::new(GOLD, BODY_TEXT_COLOR),
        );
        push_section(
            &mut lines,
            "── Grift ──",
            DESCRIPTION,
            SectionStyle::new(COPPER, BODY_TEXT_COLOR),
        );

        // ── Links ──
        lines.push(Line::styled(
            "── Links ──",
            Style::default().fg(SILVER).bold(),
        ));
        lines.push(Line::styled(
            "────────────",
            Style::default().fg(Color::Rgb(140, 145, 155)),
        ));

        // Link lines: each link label is on its own line, followed by a description line
        // We track the logical line index where each link label appears for click tracking
        let mut link_line_indices: Vec<usize> = Vec::new();
        for (label, _url, desc) in LINKS.iter() {
            link_line_indices.push(lines.len());
            lines.push(Line::styled(
                format!("  {label}"),
                Style::default().fg(LINK_TEXT_COLOR),
            ));
            lines.push(Line::styled(
                format!("    — {desc}"),
                Style::default().fg(SUBTLE_TEXT_COLOR),
            ));
        }

        push_blank_line(&mut lines);
        push_section(
            &mut lines,
            "── This Website ──",
            "Everything you see is a Rust terminal UI compiled to WebAssembly and rendered to an HTML canvas via Ratzilla. TachyonFX provides the animated background, page transitions, and hover effects. There is no HTML layout, no CSS styling, and no JavaScript framework — just a Rust application drawing characters to a terminal grid.",
            SectionStyle::new(GOLD, BODY_TEXT_COLOR),
        );
        push_section(
            &mut lines,
            "── How It Works ──",
            "Traditional web apps use HTML/CSS/JavaScript to render DOM elements. This site takes a different approach: the entire UI is a Rust application compiled to WASM, rendering a terminal grid to an HTML canvas. There is no DOM manipulation, no CSS layout engine, and no JavaScript framework involved.",
            SectionStyle::new(COPPER, BODY_TEXT_COLOR),
        );

        lines.push(Line::styled(
            "── Built With ──",
            Style::default().fg(SILVER).bold(),
        ));
        push_blank_line(&mut lines);
        push_bullet_list(
            &mut lines,
            &[
                "  • Ratzilla — terminal web apps with Rust + WASM",
                "  • Ratatui — terminal UI framework for Rust",
                "  • TachyonFX — shader-like effects for terminal UIs",
                "  • Grift — minimalistic Lisp with vau calculus",
                "  • WebGL2 rendering at 60fps on modern devices",
            ],
            BODY_TEXT_COLOR,
        );
        push_blank_line(&mut lines);

        lines.push(Line::styled(
            "── Interactions ──",
            Style::default().fg(GOLD).bold(),
        ));
        push_blank_line(&mut lines);
        push_bullet_list(
            &mut lines,
            &[
                "  • Swipe LEFT / RIGHT to switch between tabs",
                "  • Swipe UP / DOWN to scroll content",
                "  • Tap on tabs, links, and buttons to interact",
                "  • Mouse wheel scrolling works everywhere",
                "  • Keyboard input works on the REPL tab",
                "  • Paste text with Ctrl+V / Cmd+V in the REPL",
            ],
            BODY_TEXT_COLOR,
        );
        push_blank_line(&mut lines);

        lines.push(Line::styled(
            "── Performance ──",
            Style::default().fg(COPPER).bold(),
        ));
        push_blank_line(&mut lines);
        push_bullet_list(
            &mut lines,
            &[
                "  • WASM binary is ~200KB compressed",
                "  • No garbage collection pauses (arena allocator)",
                "  • Minimal memory footprint — fixed arena with const generics",
                "  • Single codebase for all screen sizes",
                "  • Full offline capability once cached by the browser",
            ],
            BODY_TEXT_COLOR,
        );

        // Precompute visual row offsets for link labels BEFORE lines is consumed by render.
        // We need the content width from the layout to do wrapped-line calculation.
        let block_for_layout = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(55, 60, 70));
        let inner_for_layout = block_for_layout.inner(area);
        let [scroll_area_for_layout, _] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(inner_for_layout);
        let content_width = scroll_area_for_layout.width.saturating_sub(2) as usize;

        // Compute visual (wrapped) row for each link label line
        let mut link_visual_rows: Vec<usize> = Vec::new();
        for &line_idx in &link_line_indices {
            let visual_row = Self::wrapped_line_count(&lines[..line_idx], content_width);
            link_visual_rows.push(visual_row);
        }

        let about_hint = if self.focus_mode == FocusMode::Focused {
            "Esc: unfocus │ ↑↓: scroll │ tap links"
        } else {
            "Enter/tap: focus │ ←→: tabs │ tap links"
        };

        // Use render_scrollable_content which handles the bordered block, scroll, and nav bar
        let mut scroll = self.about_scroll;
        self.render_scroll_page(
            frame,
            area,
            lines,
            &mut scroll,
            about_hint,
            Some(" About ".bold().fg(GOLD).into()),
            Some(" gold.silver.copper ".bold().fg(COPPER).into()),
        );
        self.about_scroll = scroll;

        // Track clickable link areas inside the scrollable view
        // The scrollable content is rendered inside a double-bordered area
        let content_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Color::Rgb(40, 44, 52));
        let content_inner = content_block.inner(scroll_area_for_layout);

        self.link_areas.clear();
        for &visual_row in &link_visual_rows {
            if visual_row >= self.about_scroll {
                let visible_row = (visual_row - self.about_scroll) as u16;
                if visible_row < content_inner.height {
                    let link_area = Rect::new(
                        content_inner.x,
                        content_inner.y + visible_row,
                        content_inner.width,
                        1,
                    );
                    self.link_areas.push(link_area);
                } else {
                    // Off-screen below: push empty rect to preserve index mapping
                    self.link_areas.push(Rect::default());
                }
            } else {
                // Off-screen above: push empty rect to preserve index mapping
                self.link_areas.push(Rect::default());
            }
        }

        // Apply hover styling to link lines (re-render hovered links with REVERSED style)
        for (i, link_area) in self.link_areas.iter().enumerate() {
            if self.is_hovered(*link_area) {
                if let Some((label, _, _)) = LINKS.get(i) {
                    let text = format!("  {label}");
                    let style = Style::default()
                        .fg(LINK_TEXT_COLOR)
                        .add_modifier(Modifier::REVERSED);
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

        lines
            .iter()
            .map(|line| {
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
                    if sym_w > max_w {
                        continue;
                    }

                    let word_found = non_ws_prev && is_ws;
                    let untrimmed_overflow = pending_empty && (word_w + ws_w + sym_w > max_w);

                    if word_found || untrimmed_overflow {
                        line_w += ws_w + word_w;
                        pending_empty = line_w == 0;
                        ws_w = 0;
                        word_w = 0;
                    }

                    let line_full = line_w >= max_w;
                    let pending_overflow = sym_w > 0 && (line_w + ws_w + word_w >= max_w);

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
            })
            .sum()
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

    fn render_scroll_arrows(
        &mut self,
        frame: &mut Frame,
        nav_bar: Rect,
        scroll: usize,
        max_scroll: usize,
        hint: &str,
    ) {
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
            Paragraph::new(" ▲ ")
                .style(up_style)
                .alignment(Alignment::Center),
            up_area,
        );
        frame.render_widget(
            Paragraph::new(" ▼ ")
                .style(down_style)
                .alignment(Alignment::Center),
            down_area,
        );

        let center_text =
            Self::scroll_hint_text(hint, scroll, max_scroll, center_area.width as usize);
        frame.render_widget(
            Paragraph::new(center_text)
                .alignment(Alignment::Center)
                .style(Style::default().fg(MUTED_TEXT_COLOR)),
            center_area,
        );
    }

    fn scroll_hint_text(hint: &str, scroll: usize, max_scroll: usize, available: usize) -> String {
        let indicator = if max_scroll > 0 {
            format!("{}/{}", scroll + 1, max_scroll + 1)
        } else {
            "─".to_string()
        };

        let full_text = format!("{hint} │ {indicator}");
        if full_text.chars().count() <= available {
            return full_text;
        }

        let segments: Vec<&str> = hint.split('│').collect();
        for drop_count in 1..segments.len() {
            let trimmed_hint = segments[drop_count..]
                .iter()
                .map(|s| s.trim())
                .collect::<Vec<_>>()
                .join(" │ ");
            let candidate = format!("{trimmed_hint} │ {indicator}");
            if candidate.chars().count() <= available {
                return candidate;
            }
        }

        indicator
    }

    /// Render a vertical scrollbar on the right border of the given area.
    /// Only renders if the content overflows (max_scroll > 0).
    fn render_vertical_scrollbar(
        &self,
        frame: &mut Frame,
        area: Rect,
        scroll: usize,
        max_scroll: usize,
    ) {
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
            self.dsl_effects_cache.resize_with(index + 1, || None);
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
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(inner);

        // Height of each effect entry: 3-row demo + title line + 1 blank
        let entry_height: u16 = 5;
        let visible_slots = (entries_area.height / entry_height).max(1) as usize;
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
            let title_area = Rect::new(entries_area.x, slot_y, entries_area.width, 1);
            let label = format!(" {:>3}. [{}] {}", idx + 1, category, title,);
            frame.render_widget(
                Paragraph::new(label).style(Style::default().fg(Color::Rgb(207, 181, 59)).bold()),
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
                Paragraph::new(code_display).style(Style::default().fg(Color::Rgb(120, 125, 140))),
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

            // Apply compiled effect to the entire entry section
            // (title + code hint + demo area) so the effect is fully visible
            let entry_area = Rect::new(
                entries_area.x,
                slot_y,
                entries_area.width,
                entry_height.min(entries_area.bottom().saturating_sub(slot_y)),
            );
            if let Some(Some(ref mut effect)) = self.dsl_effects_cache.get_mut(idx) {
                frame.render_effect(effect, entry_area, elapsed);
            }
        }

        // ── nav bar ─────────────────────────────────────────────────────
        let effects_hint = if self.focus_mode == FocusMode::Focused {
            "Esc: unfocus │ ↑↓: scroll"
        } else {
            "Enter/tap: focus │ ←→: tabs │ ↑↓: scroll"
        };
        self.render_scroll_arrows(
            frame,
            nav_bar,
            self.dsl_effects_scroll,
            max_scroll,
            effects_hint,
        );
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
    let js = format!(
        "setTimeout(function(){{window.open('{}','_blank','noopener')}},500)",
        escaped
    );
    let _ = web_sys::js_sys::eval(&js);
}

fn keycode_to_virtual_key(code: &KeyCode) -> Option<VirtualKey> {
    match code {
        KeyCode::Char(c) => match c.to_ascii_lowercase() {
            'a' => Some(VirtualKey::KeyA),
            'b' => Some(VirtualKey::KeyB),
            'c' => Some(VirtualKey::KeyC),
            'd' => Some(VirtualKey::KeyD),
            'e' => Some(VirtualKey::KeyE),
            'f' => Some(VirtualKey::KeyF),
            'g' => Some(VirtualKey::KeyG),
            'h' => Some(VirtualKey::KeyH),
            'i' => Some(VirtualKey::KeyI),
            'j' => Some(VirtualKey::KeyJ),
            'k' => Some(VirtualKey::KeyK),
            'l' => Some(VirtualKey::KeyL),
            'm' => Some(VirtualKey::KeyM),
            'n' => Some(VirtualKey::KeyN),
            'o' => Some(VirtualKey::KeyO),
            'p' => Some(VirtualKey::KeyP),
            'q' => Some(VirtualKey::KeyQ),
            'r' => Some(VirtualKey::KeyR),
            's' => Some(VirtualKey::KeyS),
            't' => Some(VirtualKey::KeyT),
            'u' => Some(VirtualKey::KeyU),
            'v' => Some(VirtualKey::KeyV),
            'w' => Some(VirtualKey::KeyW),
            'x' => Some(VirtualKey::KeyX),
            'y' => Some(VirtualKey::KeyY),
            'z' => Some(VirtualKey::KeyZ),
            '0' => Some(VirtualKey::Num0),
            '1' => Some(VirtualKey::Num1),
            '2' => Some(VirtualKey::Num2),
            '3' => Some(VirtualKey::Num3),
            '4' => Some(VirtualKey::Num4),
            '5' => Some(VirtualKey::Num5),
            '6' => Some(VirtualKey::Num6),
            '7' => Some(VirtualKey::Num7),
            '8' => Some(VirtualKey::Num8),
            '9' => Some(VirtualKey::Num9),
            ' ' => Some(VirtualKey::Space),
            _ => None,
        },
        KeyCode::Enter => Some(VirtualKey::Return),
        KeyCode::Backspace => Some(VirtualKey::Backspace),
        KeyCode::Tab => Some(VirtualKey::Tab),
        KeyCode::Esc => Some(VirtualKey::Escape),
        _ => None,
    }
}

fn main() -> std::io::Result<()> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let app = Rc::new(RefCell::new(App::new()));

    // Use WebGl2Backend with mouse selection for text select and copy/paste.
    macro_rules! setup_terminal {
        ($terminal:expr, $app:expr) => {{
            $terminal
                .on_key_event({
                    let app = $app.clone();
                    move |key_event| {
                        app.borrow_mut().handle_key_event(key_event);
                    }
                })
                .expect("failed to register key event handler");
            $terminal
                .on_mouse_event({
                    let app = $app.clone();
                    move |mouse_event| {
                        app.borrow_mut().handle_mouse_event(mouse_event);
                    }
                })
                .expect("failed to register mouse event handler");
            $terminal.draw_web({
                let app = $app.clone();
                move |frame| {
                    app.borrow_mut().draw(frame);
                }
            });
        }};
    }

    let options = WebGl2BackendOptions::new().enable_mouse_selection_with_mode(Default::default());
    let backend =
        WebGl2Backend::new_with_options(options).expect("failed to create WebGL2 backend");
    let mut terminal = ratzilla::ratatui::Terminal::new(backend)?;
    setup_terminal!(terminal, app);

    Ok(())
}
