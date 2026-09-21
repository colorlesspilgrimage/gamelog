use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use crossterm::event::KeyCode;
use serde::{Deserialize, Serialize};

use crate::paths::gamelog_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Keybindings {
    pub move_up: KeyCode,
    pub move_down: KeyCode,
    pub filter_next: KeyCode,
    pub filter_prev: KeyCode,
    pub edit_notes: KeyCode,
    pub add_entry: KeyCode,
    pub delete_entry: KeyCode,
    pub cycle_status: KeyCode,
    pub edit_hours: KeyCode,
    pub set_rating: KeyCode,
    pub import_cover: KeyCode,
    pub edit_title: KeyCode,
    pub edit_system: KeyCode,
    pub edit_release_date: KeyCode,
    pub fetch_all_covers: KeyCode,
    pub quit: KeyCode,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            move_up: KeyCode::Char('k'),
            move_down: KeyCode::Char('j'),
            filter_next: KeyCode::Tab,
            filter_prev: KeyCode::BackTab,
            edit_notes: KeyCode::Enter,
            add_entry: KeyCode::Char('a'),
            delete_entry: KeyCode::Char('d'),
            cycle_status: KeyCode::Char('s'),
            edit_hours: KeyCode::Char('h'),
            set_rating: KeyCode::Char('r'),
            import_cover: KeyCode::Char('c'),
            edit_title: KeyCode::Char('T'),
            edit_system: KeyCode::Char('p'),
            edit_release_date: KeyCode::Char('R'),
            fetch_all_covers: KeyCode::Char('F'),
            quit: KeyCode::Char('q'),
        }
    }
}

const TEMPLATE: &str = r#"# gamelog keybindings
#
# Each line maps an action to a key. A single character is written as
# { Char = "j" }; special keys are plain strings: "Tab", "BackTab", "Enter",
# "Esc", "Up", "Down", "Left", "Right", "Backspace", "Delete".
#
# An uppercase letter means Shift is held, e.g. { Char = "T" } is Shift+t.
#
# Up/Down arrows and Esc always work as fixed fallbacks for navigating and
# quitting, no matter what you bind below. Any action you leave out here
# keeps its default. Restart gamelog after editing for changes to apply.

move_up = { Char = "k" }
move_down = { Char = "j" }
filter_next = "Tab"
filter_prev = "BackTab"
edit_notes = "Enter"
add_entry = { Char = "a" }
delete_entry = { Char = "d" }
cycle_status = { Char = "s" }
edit_hours = { Char = "h" }
set_rating = { Char = "r" }
import_cover = { Char = "c" }
edit_title = { Char = "T" }
edit_system = { Char = "p" }
edit_release_date = { Char = "R" }
fetch_all_covers = { Char = "F" }
quit = { Char = "q" }
"#;

impl Keybindings {
    fn config_path() -> Result<PathBuf> {
        Ok(gamelog_dir()?.join("keybindings.toml"))
    }

    /// Loads keybindings from disk, writing out a commented default file on
    /// first run so the user has something to edit. Returns a warning
    /// message if the loaded config has a conflicting duplicate binding.
    pub fn load() -> Result<(Self, Option<String>)> {
        let path = Self::config_path()?;
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("creating config directory at {}", parent.display())
                })?;
            }
            fs::write(&path, TEMPLATE)
                .with_context(|| format!("writing default keybindings to {}", path.display()))?;
            return Ok((Self::default(), None));
        }

        let raw =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let keybindings: Self =
            toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        let warning = keybindings.duplicate_warning();
        Ok((keybindings, warning))
    }

    fn bindings(&self) -> [(&'static str, KeyCode); 16] {
        [
            ("move_up", self.move_up),
            ("move_down", self.move_down),
            ("filter_next", self.filter_next),
            ("filter_prev", self.filter_prev),
            ("edit_notes", self.edit_notes),
            ("add_entry", self.add_entry),
            ("delete_entry", self.delete_entry),
            ("cycle_status", self.cycle_status),
            ("edit_hours", self.edit_hours),
            ("set_rating", self.set_rating),
            ("import_cover", self.import_cover),
            ("edit_title", self.edit_title),
            ("edit_system", self.edit_system),
            ("edit_release_date", self.edit_release_date),
            ("fetch_all_covers", self.fetch_all_covers),
            ("quit", self.quit),
        ]
    }

    /// Detects two actions bound to the same key, which would leave one of
    /// them unreachable.
    fn duplicate_warning(&self) -> Option<String> {
        let bindings = self.bindings();
        for i in 0..bindings.len() {
            for j in (i + 1)..bindings.len() {
                if bindings[i].1 == bindings[j].1 {
                    return Some(format!(
                        "Keybinding conflict: '{}' is bound to both {} and {}.",
                        key_label(bindings[i].1),
                        bindings[i].0,
                        bindings[j].0
                    ));
                }
            }
        }
        None
    }
}

/// A short display label for a key, used in the footer help bar.
pub fn key_label(code: KeyCode) -> String {
    match code {
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => "S-Tab".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        other => format!("{other:?}"),
    }
}
