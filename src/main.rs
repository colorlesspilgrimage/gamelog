mod app;
mod entry;
mod storage;
mod ui;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui_image::picker::Picker;

use app::{App, Mode, TextInputKind};
use storage::Library;

fn main() -> Result<()> {
    let library = Library::load()?;

    let terminal = ratatui::init();
    // Must run after entering the alternate screen but before reading events.
    let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());

    let mut app = App::new(library, picker);
    let result = run(terminal, &mut app);

    ratatui::restore();
    result
}

fn run(mut terminal: ratatui::DefaultTerminal, app: &mut App) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            app.clear_status_message();

            match app.mode.clone() {
                Mode::Normal => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                    KeyCode::Up | KeyCode::Char('k') => app.select_previous(),
                    KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                    KeyCode::Tab => app.cycle_filter_next(),
                    KeyCode::BackTab => app.cycle_filter_previous(),
                    KeyCode::Enter => app.begin_edit_notes(),
                    KeyCode::Char('a') => app.begin_add_entry(),
                    KeyCode::Char('d') => app.begin_delete(),
                    KeyCode::Char('s') => app.cycle_status()?,
                    KeyCode::Char('h') => app.begin_edit_hours(),
                    KeyCode::Char('r') => app.begin_rating(),
                    KeyCode::Char('c') => app.begin_import_cover(),
                    KeyCode::Char('T') => app.begin_edit_title(),
                    KeyCode::Char('p') => app.begin_edit_system(),
                    KeyCode::Char('R') => app.begin_edit_release_date(),
                    _ => {}
                },
                Mode::EditingNotes => match key.code {
                    KeyCode::Esc => app.commit_notes()?,
                    KeyCode::Enter => app.input_buffer.push('\n'),
                    KeyCode::Backspace => {
                        app.input_buffer.pop();
                    }
                    KeyCode::Char(c) => app.input_buffer.push(c),
                    _ => {}
                },
                Mode::ConfirmDelete => match key.code {
                    KeyCode::Char('y') => app.confirm_delete()?,
                    KeyCode::Char('n') | KeyCode::Esc => app.cancel_input(),
                    _ => {}
                },
                Mode::AwaitingRating => match key.code {
                    KeyCode::Esc => app.cancel_input(),
                    KeyCode::Char('u') | KeyCode::Backspace => app.clear_rating()?,
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        let digit = c.to_digit(10).unwrap() as u8;
                        if digit <= 5 {
                            app.set_rating(digit)?;
                        }
                    }
                    _ => {}
                },
                Mode::TextInput(kind) => match key.code {
                    KeyCode::Esc => app.cancel_input(),
                    KeyCode::Backspace => {
                        app.input_buffer.pop();
                    }
                    KeyCode::Char(c) => app.input_buffer.push(c),
                    KeyCode::Enter => match kind {
                        TextInputKind::NewTitle => app.submit_new_title(),
                        TextInputKind::NewSystem => app.submit_new_system()?,
                        TextInputKind::EditTitle => app.commit_title()?,
                        TextInputKind::EditSystem => app.commit_system()?,
                        TextInputKind::EditHours => app.commit_hours()?,
                        TextInputKind::EditReleaseDate => app.commit_release_date()?,
                        TextInputKind::ImportCoverArt => app.commit_import_cover()?,
                    },
                    _ => {}
                },
            }
        }
    }

    Ok(())
}
