use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use image::ImageReader;
use ratatui::widgets::ListState;
use ratatui_image::{picker::Picker, protocol::StatefulProtocol};
use uuid::Uuid;

use crate::cover_fetch;
use crate::entry::{Entry, Rating};
use crate::keybindings::Keybindings;
use crate::settings::{CoverSource, Settings};
use crate::storage::Library;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Filter {
    All,
    System(String),
}

impl Filter {
    pub fn label(&self) -> &str {
        match self {
            Filter::All => "All Entries",
            Filter::System(system) => system,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextInputKind {
    NewTitle,
    NewSystem,
    EditTitle,
    EditSystem,
    EditHours,
    EditReleaseDate,
    ImportCoverArt,
}

impl TextInputKind {
    pub fn title(self) -> &'static str {
        match self {
            TextInputKind::NewTitle => " New Entry: Title ",
            TextInputKind::NewSystem => " New Entry: System ",
            TextInputKind::EditTitle => " Edit Title ",
            TextInputKind::EditSystem => " Edit System ",
            TextInputKind::EditHours => " Edit Hours ",
            TextInputKind::EditReleaseDate => " Edit Release Date ",
            TextInputKind::ImportCoverArt => " Import Cover Art ",
        }
    }

    pub fn prompt(self) -> &'static str {
        match self {
            TextInputKind::NewTitle => "Enter the game's title:",
            TextInputKind::NewSystem => "Enter the system/platform:",
            TextInputKind::EditTitle => "Enter the game's title:",
            TextInputKind::EditSystem => "Enter the system/platform:",
            TextInputKind::EditHours => "Enter hours played:",
            TextInputKind::EditReleaseDate => "Enter date as YYYY-MM-DD (blank to clear):",
            TextInputKind::ImportCoverArt => "Enter path to an image file:",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    EditingNotes,
    ConfirmDelete,
    AwaitingRating,
    TextInput(TextInputKind),
    /// First-run wizard: choose a default cover art source.
    SetupChooseSource,
    /// Prompting for an API key. `entry_id: None` means this is part of the
    /// first-run wizard; `Some(id)` means it's a mid-fallback retry for a
    /// specific entry, and `tried` carries forward the sources already
    /// attempted for that entry so Esc can restore `CoverFetchChoice`.
    EnterApiKey {
        source: CoverSource,
        entry_id: Option<Uuid>,
        tried: Vec<CoverSource>,
    },
    /// Shown after an automatic cover art fetch fails, offering the user an
    /// alternative source, manual import, or no cover art at all.
    CoverFetchChoice {
        entry_id: Uuid,
        tried: Vec<CoverSource>,
    },
}

pub struct App {
    pub library: Library,
    pub filter: Filter,
    pub list_state: ListState,
    pub mode: Mode,
    pub input_buffer: String,
    pub status_message: Option<String>,
    pub picker: Picker,
    pub keybindings: Keybindings,
    pub settings: Settings,
    pub should_quit: bool,
    /// Set right after creating an entry when it should be auto-fetched;
    /// drained by the event loop between draws so a "Fetching..." message
    /// is visible before the blocking network call runs.
    pub pending_cover_fetch: Option<(Uuid, CoverSource)>,
    pending_new_title: Option<String>,
    image_cache: HashMap<Uuid, Option<StatefulProtocol>>,
}

impl App {
    pub fn new(library: Library, picker: Picker, keybindings: Keybindings, settings: Settings) -> Self {
        let mut app = Self {
            library,
            filter: Filter::All,
            list_state: ListState::default(),
            mode: Mode::Normal,
            input_buffer: String::new(),
            status_message: None,
            picker,
            keybindings,
            settings,
            should_quit: false,
            pending_cover_fetch: None,
            pending_new_title: None,
            image_cache: HashMap::new(),
        };
        if !app.visible_entries().is_empty() {
            app.list_state.select(Some(0));
        }
        app
    }

    /// Systems present in the library, sorted and de-duplicated, for cycling the filter.
    pub fn systems(&self) -> Vec<String> {
        let mut systems: Vec<String> = self
            .library
            .entries()
            .iter()
            .map(|e| e.system.clone())
            .collect();
        systems.sort();
        systems.dedup();
        systems
    }

    /// Entries visible under the current filter, sorted by title.
    pub fn visible_entries(&self) -> Vec<&Entry> {
        let mut entries: Vec<&Entry> = self
            .library
            .entries()
            .iter()
            .filter(|e| match &self.filter {
                Filter::All => true,
                Filter::System(system) => &e.system == system,
            })
            .collect();
        entries.sort_by(|a, b| a.title.cmp(&b.title));
        entries
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        let entries = self.visible_entries();
        self.list_state.selected().and_then(|i| entries.get(i).copied())
    }

    fn clamp_selection(&mut self) {
        let len = self.visible_entries().len();
        if len == 0 {
            self.list_state.select(None);
        } else {
            let selected = self.list_state.selected().unwrap_or(0).min(len - 1);
            self.list_state.select(Some(selected));
        }
    }

    fn select_by_id(&mut self, id: Uuid) {
        if let Some(idx) = self.visible_entries().iter().position(|e| e.id == id) {
            self.list_state.select(Some(idx));
        }
    }

    pub fn select_next(&mut self) {
        let len = self.visible_entries().len();
        if len == 0 {
            return;
        }
        let next = match self.list_state.selected() {
            Some(i) => (i + 1).min(len - 1),
            None => 0,
        };
        self.list_state.select(Some(next));
    }

    pub fn select_previous(&mut self) {
        if self.visible_entries().is_empty() {
            return;
        }
        let prev = match self.list_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.list_state.select(Some(prev));
    }

    pub fn cycle_filter_next(&mut self) {
        let systems = self.systems();
        self.filter = match &self.filter {
            Filter::All => systems.into_iter().next().map_or(Filter::All, Filter::System),
            Filter::System(current) => {
                let idx = systems.iter().position(|s| s == current);
                match idx {
                    Some(i) if i + 1 < systems.len() => Filter::System(systems[i + 1].clone()),
                    _ => Filter::All,
                }
            }
        };
        self.clamp_selection();
        if self.list_state.selected().is_none() && !self.visible_entries().is_empty() {
            self.list_state.select(Some(0));
        }
    }

    pub fn cycle_filter_previous(&mut self) {
        let systems = self.systems();
        self.filter = match &self.filter {
            Filter::All => systems.into_iter().last().map_or(Filter::All, Filter::System),
            Filter::System(current) => {
                let idx = systems.iter().position(|s| s == current);
                match idx {
                    Some(0) | None => Filter::All,
                    Some(i) => Filter::System(systems[i - 1].clone()),
                }
            }
        };
        self.clamp_selection();
        if self.list_state.selected().is_none() && !self.visible_entries().is_empty() {
            self.list_state.select(Some(0));
        }
    }

    pub fn clear_status_message(&mut self) {
        self.status_message = None;
    }

    fn cancel_to_normal(&mut self) {
        self.mode = Mode::Normal;
        self.input_buffer.clear();
        self.pending_new_title = None;
    }

    // --- Notes ---

    pub fn begin_edit_notes(&mut self) {
        if let Some(entry) = self.selected_entry() {
            self.input_buffer = entry.notes.clone();
            self.mode = Mode::EditingNotes;
        }
    }

    pub fn commit_notes(&mut self) -> Result<()> {
        if let Some(id) = self.selected_entry().map(|e| e.id) {
            if let Some(entry) = self.library.get_mut(id) {
                entry.notes = std::mem::take(&mut self.input_buffer);
            }
            self.library.save()?;
        }
        self.mode = Mode::Normal;
        Ok(())
    }

    // --- Add entry ---

    pub fn begin_add_entry(&mut self) {
        self.input_buffer.clear();
        self.pending_new_title = None;
        self.mode = Mode::TextInput(TextInputKind::NewTitle);
    }

    pub fn submit_new_title(&mut self) {
        let title = self.input_buffer.trim().to_string();
        if title.is_empty() {
            self.status_message = Some("Title cannot be empty.".to_string());
            return;
        }
        self.pending_new_title = Some(title);
        self.input_buffer.clear();
        self.mode = Mode::TextInput(TextInputKind::NewSystem);
    }

    pub fn submit_new_system(&mut self) -> Result<()> {
        let system = self.input_buffer.trim().to_string();
        if system.is_empty() {
            self.status_message = Some("System cannot be empty.".to_string());
            return Ok(());
        }
        let title = self.pending_new_title.take().unwrap_or_default();
        let entry = Entry::new(title, system);
        let id = entry.id;
        self.library.add(entry);
        self.library.save()?;
        self.input_buffer.clear();
        self.select_by_id(id);

        match self.settings.default_cover_source {
            CoverSource::Manual => {
                self.mode = Mode::Normal;
            }
            source => {
                if self.settings.api_key_for(source).is_some() {
                    self.pending_cover_fetch = Some((id, source));
                    self.status_message = Some(format!("Fetching cover art from {}...", source.label()));
                    self.mode = Mode::Normal;
                } else {
                    // Shouldn't normally happen, since choosing an online
                    // default requires providing a key, but handle it
                    // gracefully rather than silently doing nothing.
                    self.mode = Mode::CoverFetchChoice { entry_id: id, tried: Vec::new() };
                }
            }
        }
        Ok(())
    }

    // --- Edit title / system ---

    pub fn begin_edit_title(&mut self) {
        if let Some(entry) = self.selected_entry() {
            self.input_buffer = entry.title.clone();
            self.mode = Mode::TextInput(TextInputKind::EditTitle);
        }
    }

    pub fn commit_title(&mut self) -> Result<()> {
        let title = self.input_buffer.trim().to_string();
        if title.is_empty() {
            self.status_message = Some("Title cannot be empty.".to_string());
            return Ok(());
        }
        if let Some(id) = self.selected_entry().map(|e| e.id) {
            if let Some(entry) = self.library.get_mut(id) {
                entry.title = title;
            }
            self.library.save()?;
            self.input_buffer.clear();
            self.mode = Mode::Normal;
            // Renaming can move the entry to a new position in the sorted list.
            self.select_by_id(id);
        }
        Ok(())
    }

    pub fn begin_edit_system(&mut self) {
        if let Some(entry) = self.selected_entry() {
            self.input_buffer = entry.system.clone();
            self.mode = Mode::TextInput(TextInputKind::EditSystem);
        }
    }

    pub fn commit_system(&mut self) -> Result<()> {
        let system = self.input_buffer.trim().to_string();
        if system.is_empty() {
            self.status_message = Some("System cannot be empty.".to_string());
            return Ok(());
        }
        if let Some(id) = self.selected_entry().map(|e| e.id) {
            if let Some(entry) = self.library.get_mut(id) {
                entry.system = system;
            }
            self.library.save()?;
            self.input_buffer.clear();
            self.mode = Mode::Normal;
            // Changing the system may remove the entry from a system-filtered view.
            self.clamp_selection();
        }
        Ok(())
    }

    // --- Release date ---

    pub fn begin_edit_release_date(&mut self) {
        if let Some(entry) = self.selected_entry() {
            self.input_buffer = entry
                .release_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            self.mode = Mode::TextInput(TextInputKind::EditReleaseDate);
        }
    }

    pub fn commit_release_date(&mut self) -> Result<()> {
        let trimmed = self.input_buffer.trim();
        let parsed = if trimmed.is_empty() {
            None
        } else {
            match chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
                Ok(date) => Some(date),
                Err(_) => {
                    self.status_message = Some("Use YYYY-MM-DD format.".to_string());
                    return Ok(());
                }
            }
        };
        if let Some(id) = self.selected_entry().map(|e| e.id) {
            if let Some(entry) = self.library.get_mut(id) {
                entry.release_date = parsed;
            }
            self.library.save()?;
        }
        self.input_buffer.clear();
        self.mode = Mode::Normal;
        Ok(())
    }

    // --- Delete entry ---

    pub fn begin_delete(&mut self) {
        if self.selected_entry().is_some() {
            self.mode = Mode::ConfirmDelete;
        }
    }

    pub fn confirm_delete(&mut self) -> Result<()> {
        if let Some(id) = self.selected_entry().map(|e| e.id) {
            self.library.remove(id);
            self.image_cache.remove(&id);
            self.library.save()?;
        }
        self.clamp_selection();
        self.mode = Mode::Normal;
        Ok(())
    }

    // --- Status ---

    pub fn cycle_status(&mut self) -> Result<()> {
        if let Some(id) = self.selected_entry().map(|e| e.id) {
            if let Some(entry) = self.library.get_mut(id) {
                entry.status = entry.status.next();
            }
            self.library.save()?;
        }
        Ok(())
    }

    // --- Hours ---

    pub fn begin_edit_hours(&mut self) {
        if let Some(entry) = self.selected_entry() {
            self.input_buffer = format!("{}", entry.hours);
            self.mode = Mode::TextInput(TextInputKind::EditHours);
        }
    }

    pub fn commit_hours(&mut self) -> Result<()> {
        match self.input_buffer.trim().parse::<f32>() {
            Ok(hours) if hours >= 0.0 => {
                if let Some(id) = self.selected_entry().map(|e| e.id) {
                    if let Some(entry) = self.library.get_mut(id) {
                        entry.hours = hours;
                    }
                    self.library.save()?;
                }
                self.input_buffer.clear();
                self.mode = Mode::Normal;
            }
            _ => {
                self.status_message = Some("Enter a number >= 0.".to_string());
            }
        }
        Ok(())
    }

    // --- Rating ---

    pub fn begin_rating(&mut self) {
        if self.selected_entry().is_some() {
            self.mode = Mode::AwaitingRating;
        }
    }

    pub fn set_rating(&mut self, stars: u8) -> Result<()> {
        if let Some(id) = self.selected_entry().map(|e| e.id) {
            if let Some(entry) = self.library.get_mut(id) {
                entry.rating = Some(Rating::new(stars));
            }
            self.library.save()?;
        }
        self.mode = Mode::Normal;
        Ok(())
    }

    pub fn clear_rating(&mut self) -> Result<()> {
        if let Some(id) = self.selected_entry().map(|e| e.id) {
            if let Some(entry) = self.library.get_mut(id) {
                entry.rating = None;
            }
            self.library.save()?;
        }
        self.mode = Mode::Normal;
        Ok(())
    }

    // --- Cover art ---

    pub fn begin_import_cover(&mut self) {
        if self.selected_entry().is_some() {
            self.input_buffer.clear();
            self.mode = Mode::TextInput(TextInputKind::ImportCoverArt);
        }
    }

    pub fn commit_import_cover(&mut self) -> Result<()> {
        let path = PathBuf::from(self.input_buffer.trim());
        if !path.is_file() {
            self.status_message = Some(format!("No such file: {}", path.display()));
            return Ok(());
        }
        let Some(id) = self.selected_entry().map(|e| e.id) else {
            self.cancel_to_normal();
            return Ok(());
        };
        match self.library.import_cover_art(id, &path) {
            Ok(relative) => {
                if let Some(entry) = self.library.get_mut(id) {
                    entry.cover_art = Some(relative);
                }
                self.image_cache.remove(&id);
                self.library.save()?;
                self.input_buffer.clear();
                self.mode = Mode::Normal;
            }
            Err(err) => {
                self.status_message = Some(format!("Import failed: {err}"));
            }
        }
        Ok(())
    }

    /// Cancels whatever text input / confirmation / rating prompt is active
    /// and returns to normal browsing.
    pub fn cancel_input(&mut self) {
        self.cancel_to_normal();
    }

    // --- First-run setup wizard ---

    pub fn choose_setup_source(&mut self, source: CoverSource) {
        if source.needs_api_key() && self.settings.api_key_for(source).is_none() {
            self.mode = Mode::EnterApiKey {
                source,
                entry_id: None,
                tried: Vec::new(),
            };
        } else {
            self.finish_setup(source);
        }
    }

    fn finish_setup(&mut self, source: CoverSource) {
        self.settings.default_cover_source = source;
        match self.settings.save() {
            Ok(()) => {
                self.status_message =
                    Some(format!("Default cover art source set to {}.", source.label()));
            }
            Err(err) => {
                self.status_message = Some(format!("Failed to save settings: {err}"));
            }
        }
        self.mode = Mode::Normal;
    }

    // --- API key entry (first run, or mid-fallback retry) ---

    pub fn submit_api_key(&mut self) -> Result<()> {
        let key = self.input_buffer.trim().to_string();
        if key.is_empty() {
            self.status_message = Some("API key cannot be empty.".to_string());
            return Ok(());
        }
        let Mode::EnterApiKey { source, entry_id, .. } = self.mode.clone() else {
            return Ok(());
        };
        self.settings.set_api_key(source, key);
        self.input_buffer.clear();

        match entry_id {
            None => self.finish_setup(source),
            Some(id) => {
                self.settings.save()?;
                self.pending_cover_fetch = Some((id, source));
                self.status_message = Some(format!("Fetching cover art from {}...", source.label()));
                self.mode = Mode::Normal;
            }
        }
        Ok(())
    }

    pub fn cancel_api_key_entry(&mut self) {
        if let Mode::EnterApiKey { entry_id, tried, .. } = self.mode.clone() {
            self.input_buffer.clear();
            self.mode = match entry_id {
                None => Mode::SetupChooseSource,
                Some(id) => Mode::CoverFetchChoice { entry_id: id, tried },
            };
        }
    }

    // --- Automatic cover art fetch ---

    /// Performs the network fetch set up by `pending_cover_fetch`. Called
    /// from the event loop between draws, so a "Fetching..." status message
    /// is visible on screen before this blocking call runs.
    pub fn run_pending_cover_fetch(&mut self) -> Result<()> {
        let Some((entry_id, source)) = self.pending_cover_fetch.take() else {
            return Ok(());
        };
        let Some(entry) = self.library.get(entry_id) else {
            return Ok(());
        };
        let title = entry.title.clone();
        let Some(api_key) = self.settings.api_key_for(source).map(str::to_string) else {
            self.mode = Mode::CoverFetchChoice { entry_id, tried: vec![source] };
            return Ok(());
        };

        match cover_fetch::fetch_cover(source, &api_key, &title) {
            Ok(Some((bytes, extension))) => {
                let relative = self.library.import_cover_art_bytes(entry_id, &bytes, &extension)?;
                if let Some(entry) = self.library.get_mut(entry_id) {
                    entry.cover_art = Some(relative);
                }
                self.image_cache.remove(&entry_id);
                self.library.save()?;
                self.status_message = Some(format!("Cover art added from {}.", source.label()));
            }
            Ok(None) => {
                self.status_message = Some(format!("No results from {}.", source.label()));
                self.mode = Mode::CoverFetchChoice { entry_id, tried: vec![source] };
            }
            Err(err) => {
                self.status_message = Some(format!("{} request failed: {err}", source.label()));
                self.mode = Mode::CoverFetchChoice { entry_id, tried: vec![source] };
            }
        }
        Ok(())
    }

    /// Attempts (or retries) fetching cover art for `entry_id` from
    /// `source`, prompting for an API key first if one isn't configured yet.
    pub fn choose_fallback_source(&mut self, entry_id: Uuid, tried: Vec<CoverSource>, source: CoverSource) {
        if self.settings.api_key_for(source).is_some() {
            self.pending_cover_fetch = Some((entry_id, source));
            self.status_message = Some(format!("Fetching cover art from {}...", source.label()));
            self.mode = Mode::Normal;
        } else {
            self.mode = Mode::EnterApiKey {
                source,
                entry_id: Some(entry_id),
                tried,
            };
        }
    }

    pub fn choose_manual_cover(&mut self, entry_id: Uuid) {
        self.select_by_id(entry_id);
        self.input_buffer.clear();
        self.mode = Mode::TextInput(TextInputKind::ImportCoverArt);
    }

    pub fn skip_cover(&mut self) {
        self.status_message = Some("No cover art added.".to_string());
        self.mode = Mode::Normal;
    }

    /// Returns the decoded, protocol-encoded cover art for `entry`, loading
    /// and caching it on first access.
    pub fn cover_protocol(&mut self, entry: &Entry) -> Option<&mut StatefulProtocol> {
        let id = entry.id;
        if !self.image_cache.contains_key(&id) {
            let loaded = entry.cover_art.as_ref().and_then(|relative| {
                let path = self.library.resolve_cover_art(relative);
                ImageReader::open(&path).ok()?.decode().ok()
            });
            let protocol = loaded.map(|img| self.picker.new_resize_protocol(img));
            self.image_cache.insert(id, protocol);
        }
        self.image_cache.get_mut(&id).and_then(|opt| opt.as_mut())
    }
}
