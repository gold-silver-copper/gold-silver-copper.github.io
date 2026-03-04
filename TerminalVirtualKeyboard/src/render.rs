use ratatui::layout::{Alignment, Constraint, Direction, Layout as TuiLayout, Rect};
use ratatui::{prelude::*, widgets::*};
use std::collections::HashSet;

use crate::env::*;
use crate::layout::Layout;
use crate::virtual_key::VirtualKey;

/// Render the keyboard inline within a given area, without outer block or KPS counter.
/// Returns a flat list of (Rect, display_name) for each button, used for click handling.
pub fn render_keyboard_inline(
    f: &mut Frame,
    area: Rect,
    pressed_keys: &HashSet<VirtualKey>,
    kbd_layout: &Layout,
    env: &Env,
) -> Vec<(Rect, String)> {
    let global_border_color = match env.get("border_color") {
        Some(Value::RGB(r, g, b)) => Color::Rgb(*r, *g, *b),
        _ => Color::Rgb(55, 60, 70),
    };

    let global_highlight = match env.get("highlight") {
        Some(Value::RGB(r, g, b)) => Color::Rgb(*r, *g, *b),
        _ => Color::Rgb(207, 181, 59),
    };

    render_rows_inline(f, area, pressed_keys, kbd_layout, env, global_border_color, global_highlight)
}

/// Render keyboard rows inline and return button areas with their active display names.
/// The keyboard is horizontally centered within the given area.
fn render_rows_inline(
    f: &mut Frame,
    area: Rect,
    pressed_keys: &HashSet<VirtualKey>,
    kbd_layout: &Layout,
    env: &Env,
    global_border_color: Color,
    global_highlight: Color,
) -> Vec<(Rect, String)> {
    let mut button_areas = Vec::new();

    let row_areas = TuiLayout::default()
        .direction(Direction::Vertical)
        .constraints(
            kbd_layout
                .layer
                .iter()
                .map(|_| Constraint::Length(3))
                .collect::<Vec<_>>(),
        )
        .split(area);

    for (r_idx, row) in kbd_layout.layer.iter().enumerate() {
        if r_idx >= row_areas.len() {
            break;
        }

        // Calculate total row width for centering
        let total_row_width: u16 = row.iter().map(|k| k.attr.width).sum();
        let row_area = row_areas[r_idx];
        let h_offset = if row_area.width > total_row_width {
            (row_area.width - total_row_width) / 2
        } else {
            0
        };

        // Create a centered sub-area for this row
        let centered_row = Rect::new(
            row_area.x + h_offset,
            row_area.y,
            total_row_width.min(row_area.width),
            row_area.height,
        );

        let key_constraints: Vec<Constraint> = row
            .iter()
            .map(|k| Constraint::Length(k.attr.width))
            .collect();

        let key_areas = TuiLayout::default()
            .direction(Direction::Horizontal)
            .constraints(key_constraints)
            .split(centered_row);

        for (k_idx, button) in row.iter().enumerate() {
            if k_idx >= key_areas.len() {
                break;
            }

            let current_border = button.attr.border_color.unwrap_or(global_border_color);
            let current_highlight = button.attr.highlight.unwrap_or(global_highlight);

            let active_bind_idx = button
                .binds
                .iter()
                .enumerate()
                .rev()
                .find(|(_, (_, key))| key.map_or(false, |k| pressed_keys.contains(&k)))
                .map(|(i, _)| i);

            let (display_name, style) = match active_bind_idx {
                Some(idx) => {
                    let name = button.binds[idx].0.as_ref();

                    let bg_color = if idx == 0 {
                        current_highlight
                    } else {
                        get_highlight(idx, env)
                    };

                    (
                        name,
                        Style::default()
                            .bg(bg_color)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    )
                }
                None => {
                    let name = button.binds.first().map(|b| b.0.as_ref()).unwrap_or("");
                    (name, Style::default().fg(Color::Rgb(170, 175, 185)).add_modifier(Modifier::BOLD))
                }
            };

            // Store the button area and its current display name
            button_areas.push((key_areas[k_idx], display_name.to_string()));

            let key_widget = Paragraph::new(display_name)
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .style(if active_bind_idx.is_some() {
                            style
                        } else {
                            Style::default().fg(current_border)
                        }),
                );

            let final_render = if active_bind_idx.is_some() {
                key_widget.style(style)
            } else {
                key_widget
            };

            f.render_widget(final_render, key_areas[k_idx]);
        }
    }

    button_areas
}

fn get_highlight(l: usize, env: &Env) -> Color {
    let default_highlight_l2 = Color::Rgb(176, 176, 176);
    let default_highlight_l3 = Color::Rgb(176, 176, 176);
    let default_highlight_other = Color::Rgb(176, 176, 176);
    match env.get(format!("highlight_l{}", l).as_str()) {
        Some(bc) => match bc {
            Value::RGB(r, g, b) => Color::Rgb(*r, *g, *b),
            _ => match l {
                1 => default_highlight_l2,
                2 => default_highlight_l3,
                _ => default_highlight_other,
            },
        },
        _ => match l {
            1 => default_highlight_l2,
            2 => default_highlight_l3,
            _ => default_highlight_other,
        },
    }
}
