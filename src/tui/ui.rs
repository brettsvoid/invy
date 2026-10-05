//! Rendering for the interactive TUI.

use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use super::app::{App, Mode, StatusKind};
use crate::model::glyph_set;

const ACCENT: Color = Color::Cyan;
const MUTED: Color = Color::DarkGray;
const MARKED: Color = Color::Magenta;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let [main, status] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(frame.area());
    let [tree_area, detail_area] =
        Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)]).areas(main);

    // Two border rows are not usable for list content.
    app.visible_rows = tree_area.height.saturating_sub(2) as usize;

    draw_tree(frame, app, tree_area);
    draw_details(frame, app, detail_area);
    draw_status(frame, app, status);

    match &app.mode {
        Mode::Prompt(prompt) => draw_prompt(frame, prompt),
        Mode::Confirm(confirm) => draw_confirm(frame, &confirm.message),
        _ => {}
    }

    if app.show_help {
        draw_help(frame);
    }
}

fn draw_tree(frame: &mut Frame, app: &App, area: Rect) {
    let title = if app.filter.is_empty() {
        let count = app.item_count();
        let plural = if count == 1 { "item" } else { "items" };
        format!(" Inventory ({count} {plural}) ")
    } else {
        format!(" Matches for '{}' ({}) ", app.filter, app.nodes.len())
    };

    let rows: Vec<ListItem> = app
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| ListItem::new(row(node, app.is_marked(index), app.is_cut(node))))
        .collect();

    let block = Block::bordered()
        .title(title)
        .border_style(Style::default().fg(MUTED));

    if rows.is_empty() {
        let message = if app.filter.is_empty() {
            "Nothing here yet. Press 'a' to add an item."
        } else {
            "No matches."
        };
        let empty = Paragraph::new(message).fg(MUTED).block(block);
        frame.render_widget(empty, area);
        return;
    }

    let list = List::new(rows)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default().with_selected(Some(app.selected));
    frame.render_stateful_widget(list, area, &mut state);
}

/// One tree row: a mark gutter, branch glyphs, an open/closed marker, the
/// name, a child count. A marked row shows in the gutter, a cut one dimmed.
fn row(node: &super::app::Node, marked: bool, cut: bool) -> Line<'static> {
    let mut spans = Vec::new();

    spans.push(if marked {
        Span::styled(glyph_set().mark(), Style::default().fg(MARKED))
    } else {
        Span::raw(" ")
    });

    if let Some(place) = &node.place_path {
        spans.push(Span::styled(place.clone(), Style::default().fg(MUTED)));
        spans.push(Span::raw(node.name.clone()));
    } else {
        let set = glyph_set();
        let tree = set.tree();

        let mut prefix = String::new();
        for has_more in &node.ancestors {
            prefix.push_str(if *has_more { tree.pipe } else { " " });
            prefix.push_str("  ");
        }
        if node.depth > 0 {
            prefix.push_str(if node.is_last { tree.elbow } else { tree.tee });
            prefix.push_str(tree.dash);
            prefix.push(' ');
        }
        spans.push(Span::styled(prefix, Style::default().fg(MUTED)));

        if node.child_count > 0 {
            let marker = if node.expanded {
                set.expanded()
            } else {
                set.collapsed()
            };
            spans.push(Span::styled(
                format!("{marker} "),
                Style::default().fg(ACCENT),
            ));
        } else {
            spans.push(Span::styled(
                format!("{} ", set.leaf()),
                Style::default().fg(MUTED),
            ));
        }

        if let Some(glyph) = node.kind.glyph() {
            spans.push(Span::styled(
                format!("{glyph} "),
                Style::default().fg(Color::Yellow),
            ));
        }

        if node.unsorted {
            spans.push(Span::styled(
                node.name.clone(),
                Style::default()
                    .fg(ACCENT)
                    .add_modifier(Modifier::ITALIC | Modifier::BOLD),
            ));
        } else if node.child_count > 0 {
            spans.push(Span::styled(
                node.name.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::raw(node.name.clone()));
        }
    }

    if node.ids.len() > 1 {
        spans.push(Span::styled(
            format!(" {}{}", glyph_set().times(), node.ids.len()),
            Style::default().fg(ACCENT),
        ));
    }

    if node.child_count > 0 {
        spans.push(Span::styled(
            format!("  ({})", node.child_count),
            Style::default().fg(MUTED),
        ));
    }

    let line = Line::from(spans);
    if cut {
        line.style(Style::default().fg(MUTED).add_modifier(Modifier::ITALIC))
    } else if marked {
        line.style(Style::default().fg(MARKED))
    } else {
        line
    }
}

fn draw_details(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::bordered()
        .title(" Details ")
        .border_style(Style::default().fg(MUTED));

    let Some(node) = app.selected_node() else {
        frame.render_widget(Paragraph::new("").block(block), area);
        return;
    };

    if node.unsorted {
        let plural = if node.child_count == 1 {
            "thing"
        } else {
            "things"
        };
        let lines = vec![
            field("Unsorted", format!("{} {plural} at root", node.child_count)),
            Line::raw(""),
            Line::raw("Where these are has not been recorded yet."),
            Line::from(Span::styled(
                "Cut them with x and paste them into a place with p, or press t on one to make it a room, furniture or a box.",
                Style::default().fg(MUTED),
            )),
        ];
        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
        return;
    }

    let path = app.path_of(node.id);
    let mut lines = vec![
        field("Name", node.name.clone()),
        field("Path", path.join(" / ")),
    ];

    lines.push(field("Kind", node.kind.to_string()));

    if node.ids.len() > 1 {
        lines.push(field("Duplicates", format!("{} here", node.ids.len())));
    }

    if node.child_count > 0 {
        let plural = if node.child_count == 1 {
            "item"
        } else {
            "items"
        };
        lines.push(field(
            "Contains",
            format!("{} {}", node.child_count, plural),
        ));
        if let Some(changed) = app.last_changed(node.id) {
            lines.push(field("Changed", changed));
        }
    }

    if let Some(item) = app.item(node.id) {
        lines.push(field("Created", item.created_at.clone()));
        lines.push(field("Updated", item.updated_at.clone()));
    }

    lines.push(Line::raw(""));
    match &node.description {
        Some(description) => {
            lines.push(Line::from(Span::styled(
                "Description",
                Style::default().fg(MUTED),
            )));
            lines.push(Line::raw(description.clone()));
        }
        None => lines.push(Line::from(Span::styled(
            "No description. Press 'e' to add one.",
            Style::default().fg(MUTED),
        ))),
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn field(label: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<10} "), Style::default().fg(MUTED)),
        Span::raw(value),
    ])
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    if let Mode::Search = app.mode {
        let line = Line::from(vec![
            Span::styled("/", Style::default().fg(ACCENT)),
            Span::raw(app.search_input.value().to_string()),
        ]);
        frame.render_widget(Paragraph::new(line), area);
        frame.set_cursor_position((area.x + 1 + app.search_input.cursor() as u16, area.y));
        return;
    }

    // What is marked or cut stays in view, whatever else the bar says.
    let mut spans = Vec::new();
    if app.visual_anchor.is_some() {
        spans.push(Span::styled(
            " VISUAL ",
            Style::default().fg(MARKED).add_modifier(Modifier::BOLD),
        ));
    }
    if !app.marks.is_empty() {
        spans.push(Span::styled(
            format!(" {} marked ", app.marks.len()),
            Style::default().fg(MARKED),
        ));
    }
    if !app.cut.is_empty() {
        spans.push(Span::styled(
            format!(" {} cut ", app.cut.len()),
            Style::default().fg(Color::Yellow),
        ));
    }

    match &app.status {
        Some((message, kind)) => {
            let colour = match kind {
                StatusKind::Info => Color::Green,
                StatusKind::Error => Color::Red,
            };
            spans.push(Span::styled(
                format!(" {message}"),
                Style::default().fg(colour),
            ));
        }
        None => spans.push(Span::styled(
            " j/k move  ⏎ fold  a/i add  +/- count  space mark  v visual  x cut  p paste  d remove  e describe  / search  ? help",
            Style::default().fg(MUTED),
        )),
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_prompt(frame: &mut Frame, prompt: &super::app::Prompt) {
    // Wide enough for the title and the hint, which can name a long path.
    let wanted = prompt
        .title
        .chars()
        .count()
        .max(prompt.hint.chars().count())
        + 4;
    let area = centred(frame.area(), wanted.max(60) as u16, 5);
    let block = Block::bordered()
        .title(format!(" {} ", prompt.title))
        .border_style(Style::default().fg(ACCENT));
    let inner = block.inner(area);

    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    let [input_area, hint_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(inner);

    frame.render_widget(Paragraph::new(prompt.input.value()), input_area);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            prompt.hint.clone(),
            Style::default().fg(MUTED),
        ))),
        hint_area,
    );

    frame.set_cursor_position((input_area.x + prompt.input.cursor() as u16, input_area.y));
}

fn draw_confirm(frame: &mut Frame, message: &str) {
    let area = centred(frame.area(), 56, 5);
    let block = Block::bordered()
        .title(" Confirm ")
        .border_style(Style::default().fg(Color::Red));
    let inner = block.inner(area);

    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::raw(message.to_string()),
        Line::raw(""),
        Line::from(Span::styled(
            "y confirm    n cancel",
            Style::default().fg(MUTED),
        )),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn draw_help(frame: &mut Frame) {
    let entries = [
        ("j / k / ↓ / ↑", "move up and down"),
        ("g / G", "first and last row"),
        ("Ctrl-d / Ctrl-u", "half page down and up"),
        ("⏎", "expand or collapse"),
        ("l / h", "expand, or collapse and go to parent"),
        ("E / C", "expand all, collapse all"),
        ("Space", "mark or unmark, then move down"),
        ("v", "visual mode: mark rows as you move"),
        ("/", "search by name or description"),
        ("Esc", "leave visual, clear marks, clear search"),
        ("a / i / A", "add beside the selection, inside it, at root"),
        ("+ / -", "add or remove a duplicate"),
        ("r", "rename the selection"),
        ("e", "edit the description"),
        ("x / X", "cut the marked items, or cancel the cut"),
        ("p", "paste the cut items into the selection"),
        ("m", "move the marked items to a typed place"),
        ("d / Del", "remove the marked items"),
        ("t / T", "next and previous kind"),
        ("R", "reload from the database"),
        ("? ", "this help"),
        ("q", "quit"),
    ];

    let area = centred(frame.area(), 60, entries.len() as u16 + 2);
    let block = Block::bordered()
        .title(" Keys ")
        .border_style(Style::default().fg(ACCENT));
    let inner = block.inner(area);

    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = entries
        .iter()
        .map(|(keys, what)| {
            Line::from(vec![
                Span::styled(format!("{keys:<17}"), Style::default().fg(ACCENT)),
                Span::raw(*what),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), inner);
}

/// A box of the given size, centred in `area` and clamped to it.
fn centred(area: Rect, width: u16, height: u16) -> Rect {
    let [row] = Layout::vertical([Constraint::Length(height.min(area.height))])
        .flex(Flex::Center)
        .areas(area);
    let [cell] = Layout::horizontal([Constraint::Length(width.min(area.width))])
        .flex(Flex::Center)
        .areas(row);
    cell
}
