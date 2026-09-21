mod app;
mod entry;
mod keybindings;
mod paths;
mod storage;
mod ui;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui_image::picker::Picker;

use app::{App, Mode, TextInputKind};
use keybindings::Keybindings;
use storage::Library;

fn main() -> Result<()> {
    let library = Library::load()?;
    let (keybindings, keybindings_warning) = Keybindings::load()?;

    let terminal = ratatui::init();
    // Must run after entering the alternate screen but before reading events.
    let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());

    let mut app = App::new(library, picker, keybindings);
    app.status_message = keybindings_warning;
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
                Mode::Normal => {
                    // Cloned so comparing against it doesn't hold a borrow of
                    // `app` while the matched arms mutate it below.
                    let kb = app.keybindings.clone();
                    match key.code {
                        // Fixed fallbacks: always available regardless of config.
                        KeyCode::Esc => app.should_quit = true,
                        KeyCode::Up => app.select_previous(),
                        KeyCode::Down => app.select_next(),
                        code if code == kb.quit => app.should_quit = true,
                        code if code == kb.move_up => app.select_previous(),
                        code if code == kb.move_down => app.select_next(),
                        code if code == kb.filter_next => app.cycle_filter_next(),
                        code if code == kb.filter_prev => app.cycle_filter_previous(),
                        code if code == kb.edit_notes => app.begin_edit_notes(),
                        code if code == kb.add_entry => app.begin_add_entry(),
                        code if code == kb.delete_entry => app.begin_delete(),
                        code if code == kb.cycle_status => app.cycle_status()?,
                        code if code == kb.edit_hours => app.begin_edit_hours(),
                        code if code == kb.set_rating => app.begin_rating(),
                        code if code == kb.import_cover => app.begin_import_cover(),
                        code if code == kb.edit_title => app.begin_edit_title(),
                        code if code == kb.edit_system => app.begin_edit_system(),
                        code if code == kb.edit_release_date => app.begin_edit_release_date(),
                        _ => {}
                    }
                }
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
