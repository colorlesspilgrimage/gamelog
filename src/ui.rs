use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use ratatui_image::StatefulImage;

use crate::app::{App, Mode, TextInputKind};
use crate::entry::{Entry, Status};
use crate::keybindings::{key_label, Keybindings};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let [main_area, footer_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).areas(frame.area());

    let [left, right] =
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
            .areas(main_area);

    draw_list(frame, app, left);
    draw_detail(frame, app, right);
    draw_footer(frame, app, footer_area);

    match app.mode.clone() {
        Mode::ConfirmDelete => draw_confirm_delete(frame, app),
        Mode::AwaitingRating => draw_rating_popup(frame, app),
        Mode::TextInput(kind) => draw_text_input_popup(frame, app, kind),
        Mode::Normal | Mode::EditingNotes => {}
    }
}

fn draw_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let title = format!(" {} ", app.filter.label());
    let items: Vec<ListItem> = app
        .visible_entries()
        .iter()
        .map(|entry| {
            ListItem::new(Line::from(vec![
                status_glyph(entry.status),
                Span::raw(" "),
                Span::raw(entry.title.clone()),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED));

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn status_glyph(status: Status) -> Span<'static> {
    match status {
        Status::WantToPlay => Span::styled("○", Style::default().fg(Color::DarkGray)),
        Status::Playing => Span::styled("◐", Style::default().fg(Color::Yellow)),
        Status::Played => Span::styled("●", Style::default().fg(Color::Green)),
    }
}

fn draw_detail(frame: &mut Frame, app: &mut App, area: Rect) {
    let [cover_area, info_area, notes_area] = Layout::vertical([
        Constraint::Percentage(45),
        Constraint::Length(8),
        Constraint::Min(5),
    ])
    .areas(area);

    let Some(entry) = app.selected_entry().cloned() else {
        let placeholder = Paragraph::new("No entries yet. Press 'a' to add one.")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(placeholder, area);
        return;
    };

    draw_cover(frame, app, &entry, cover_area);
    draw_info(frame, &entry, info_area);
    draw_notes(frame, app, notes_area);
}

fn draw_cover(frame: &mut Frame, app: &mut App, entry: &Entry, area: Rect) {
    let block = Block::default().title(" Cover Art ").borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    match app.cover_protocol(entry) {
        Some(protocol) => {
            frame.render_stateful_widget(StatefulImage::default(), inner, protocol);
        }
        None => {
            let placeholder = Paragraph::new("No cover art (press 'c' to import)")
                .alignment(Alignment::Center);
            frame.render_widget(placeholder, inner);
        }
    }
}

fn draw_info(frame: &mut Frame, entry: &Entry, area: Rect) {
    let release_date = entry
        .release_date
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let rating = entry
        .rating
        .map(|r| stars(r.stars()))
        .unwrap_or_else(|| "Unrated".to_string());

    let status_style = match entry.status {
        Status::WantToPlay => Style::default().fg(Color::DarkGray),
        Status::Playing => Style::default().fg(Color::Yellow),
        Status::Played => Style::default().fg(Color::Green),
    };

    let lines = vec![
        Line::from(Span::styled(
            entry.title.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("System: {}", entry.system)),
        Line::from(format!("Released: {release_date}")),
        Line::from(vec![
            Span::raw("Status: "),
            Span::styled(entry.status.label(), status_style),
        ]),
        Line::from(format!("Hours: {:.1}", entry.hours)),
        Line::from(format!("Rating: {rating}")),
    ];

    let info = Paragraph::new(lines).block(Block::default().title(" Info ").borders(Borders::ALL));
    frame.render_widget(info, area);
}

fn stars(count: u8) -> String {
    let filled = "★".repeat(count as usize);
    let empty = "☆".repeat((5u8.saturating_sub(count)) as usize);
    format!("{filled}{empty}")
}

fn draw_notes(frame: &mut Frame, app: &App, area: Rect) {
    let editing = matches!(app.mode, Mode::EditingNotes);
    let title = if editing {
        " Notes (editing - Esc to save) "
    } else {
        " Notes (Enter to edit) "
    };

    let text = if editing {
        app.input_buffer.as_str()
    } else {
        app.selected_entry().map(|e| e.notes.as_str()).unwrap_or("")
    };

    let border_style = if editing {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);

    if editing {
        let cursor_line = text.split('\n').next_back().unwrap_or("");
        let row = text.matches('\n').count() as u16;
        let col = cursor_line.chars().count() as u16;
        frame.set_cursor_position((inner.x + col, inner.y + row));
    }
}

/// Keybinding hints for the currently active mode, shown along the bottom.
/// Normal-mode hints reflect the user's configured keybindings; the other
/// modes use fixed dialog conventions (Enter/Esc/digits) that aren't
/// user-configurable.
fn footer_hints(mode: &Mode, kb: &Keybindings) -> Vec<(String, &'static str)> {
    match mode {
        Mode::Normal => vec![
            (
                format!("{}/{}", key_label(kb.move_down), key_label(kb.move_up)),
                "Navigate",
            ),
            (
                format!("{}/{}", key_label(kb.filter_next), key_label(kb.filter_prev)),
                "Filter",
            ),
            (key_label(kb.edit_notes), "Notes"),
            (key_label(kb.add_entry), "Add"),
            (key_label(kb.delete_entry), "Delete"),
            (key_label(kb.edit_title), "Title"),
            (key_label(kb.edit_system), "Platform"),
            (key_label(kb.edit_release_date), "Release Date"),
            (key_label(kb.cycle_status), "Status"),
            (key_label(kb.edit_hours), "Hours"),
            (key_label(kb.set_rating), "Rating"),
            (key_label(kb.import_cover), "Cover Art"),
            (key_label(kb.quit), "Quit"),
        ],
        Mode::EditingNotes => vec![
            ("Enter".to_string(), "Newline"),
            ("Esc".to_string(), "Save & Exit"),
        ],
        Mode::ConfirmDelete => vec![
            ("y".to_string(), "Confirm"),
            ("n/Esc".to_string(), "Cancel"),
        ],
        Mode::AwaitingRating => vec![
            ("0-5".to_string(), "Set Stars"),
            ("u/Backspace".to_string(), "Unrate"),
            ("Esc".to_string(), "Cancel"),
        ],
        Mode::TextInput(kind) => {
            let enter_label = match kind {
                TextInputKind::NewTitle => "Next: System",
                TextInputKind::NewSystem => "Create Entry",
                TextInputKind::ImportCoverArt => "Import",
                _ => "Save",
            };
            vec![
                ("Enter".to_string(), enter_label),
                ("Esc".to_string(), "Cancel"),
            ]
        }
    }
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let line = if let Some(message) = &app.status_message {
        Line::from(Span::styled(
            message.clone(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ))
    } else {
        let mut spans = Vec::new();
        for (i, (key, action)) in footer_hints(&app.mode, &app.keybindings).into_iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw("  "));
            }
            spans.push(Span::styled(
                format!(" {key} "),
                Style::default().fg(Color::Black).bg(Color::Gray),
            ));
            spans.push(Span::raw(format!(" {action}")));
        }
        Line::from(spans)
    };

    let paragraph = Paragraph::new(line).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

/// A rectangle of `percent_x` x `percent_y` centered within `area`.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [_, vertical, _] = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .areas(area);
    let [_, horizontal, _] = Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .areas(vertical);
    horizontal
}

fn draw_popup(frame: &mut Frame, title: &str, lines: Vec<Line>, area: Rect) {
    let popup_area = centered_rect(50, 30, area);
    frame.render_widget(Clear, popup_area);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, popup_area);
}

fn draw_confirm_delete(frame: &mut Frame, app: &App) {
    let title_text = app
        .selected_entry()
        .map(|e| e.title.clone())
        .unwrap_or_default();
    let lines = vec![
        Line::from(format!("Delete '{title_text}'?")),
        Line::from("This cannot be undone."),
    ];
    draw_popup(frame, " Confirm Delete ", lines, frame.area());
}

fn draw_rating_popup(frame: &mut Frame, app: &App) {
    let current = app
        .selected_entry()
        .and_then(|e| e.rating)
        .map(|r| stars(r.stars()))
        .unwrap_or_else(|| "Unrated".to_string());
    let lines = vec![
        Line::from(format!("Current rating: {current}")),
        Line::from("Press 0-5 to set stars, u to unset."),
    ];
    draw_popup(frame, " Set Rating ", lines, frame.area());
}

fn draw_text_input_popup(frame: &mut Frame, app: &App, kind: TextInputKind) {
    let popup_area = centered_rect(50, 30, frame.area());
    frame.render_widget(Clear, popup_area);
    let block = Block::default()
        .title(kind.title())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let [prompt_area, input_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(inner);

    frame.render_widget(Paragraph::new(kind.prompt()), prompt_area);
    frame.render_widget(Paragraph::new(app.input_buffer.as_str()), input_area);
    frame.set_cursor_position((
        input_area.x + app.input_buffer.chars().count() as u16,
        input_area.y,
    ));
}
