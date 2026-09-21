use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use eframe::egui;
use uuid::Uuid;

use crate::app::{App, Filter, Mode, TextInputKind};
use crate::keybindings::Keybindings;
use crate::settings::{CoverSource, Settings};
use crate::storage::Library;
use crate::ui::{format_duration, stars};

/// Launches the egui-based GUI as an alternative to the terminal UI. It
/// shares the same `App` state machine as the TUI (add/edit/delete, cover
/// art fetching, sessions, the first-run wizard, ...) so the two front ends
/// never drift in behavior; only the rendering and input handling differ.
pub fn run(library: Library, keybindings: Keybindings, settings: Settings, first_run: bool) -> Result<()> {
    // The picker is only ever used for ratatui's terminal image protocols;
    // the GUI renders cover art itself via egui textures, so any picker works.
    let picker = ratatui_image::picker::Picker::halfblocks();
    let mut app = App::new(library, picker, keybindings, settings);
    if first_run {
        app.mode = Mode::SetupChooseSource;
    }
    let gui_app = GamelogApp { app, textures: HashMap::new(), last_text_input_kind: None };
    eframe::run_native(
        "gamelog",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(gui_app))),
    )
    .map_err(|err| anyhow::anyhow!("failed to launch GUI: {err}"))
}

struct GamelogApp {
    app: App,
    textures: HashMap<Uuid, Option<egui::TextureHandle>>,
    /// The `TextInputKind` last focused, so switching from one text-input
    /// dialog straight into another (e.g. new entry's title -> system) grabs
    /// focus for the new field instead of leaving it on the previous
    /// dialog's now-stale OK button.
    last_text_input_kind: Option<TextInputKind>,
}

impl GamelogApp {
    /// Surfaces a fallible `App` action's error as a status message instead
    /// of unwinding: unlike the TUI's event loop, egui's `ui()` can't
    /// propagate a `Result`, and a save failure shouldn't take the whole
    /// window down with it.
    fn report(&mut self, result: Result<()>) {
        if let Err(err) = result {
            self.app.status_message = Some(format!("Error: {err}"));
        }
    }

    /// Cover fetches complete on a background thread; rather than track
    /// exactly which entry changed, just drop the whole texture cache once a
    /// fetch finishes and let the next frame reload lazily.
    fn poll_and_invalidate(&mut self) {
        let was_fetching = self.app.bulk_fetch_progress.is_some();
        let result = self.app.poll_cover_fetch_results();
        self.report(result);
        if was_fetching && self.app.bulk_fetch_progress.is_none() {
            self.textures.clear();
        }
    }

    fn cover_texture(&mut self, ctx: &egui::Context, entry_id: Uuid) -> Option<egui::TextureHandle> {
        if !self.textures.contains_key(&entry_id) {
            let loaded = self.app.library.get(entry_id).and_then(|entry| {
                let relative = entry.cover_art.as_ref()?;
                let path = self.app.library.resolve_cover_art(relative);
                let img = image::ImageReader::open(&path).ok()?.decode().ok()?;
                let rgba = img.to_rgba8();
                let (width, height) = rgba.dimensions();
                let color_image =
                    egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], rgba.as_raw());
                Some(ctx.load_texture(format!("cover-{entry_id}"), color_image, egui::TextureOptions::LINEAR))
            });
            self.textures.insert(entry_id, loaded);
        }
        self.textures.get(&entry_id).cloned().flatten()
    }

    /// Mirrors `main.rs`'s per-mode Escape handling, so the key does the same
    /// thing in both front ends.
    fn handle_escape(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            return;
        }
        let result = match self.app.mode.clone() {
            Mode::Normal => self.app.quit(),
            Mode::EditingNotes => self.app.commit_notes(),
            Mode::ConfirmDelete | Mode::AwaitingRating | Mode::TextInput(_) => {
                self.app.cancel_input();
                Ok(())
            }
            Mode::SetupChooseSource => {
                self.app.choose_setup_source(CoverSource::Manual);
                Ok(())
            }
            Mode::EnterApiKey { .. } => {
                self.app.cancel_api_key_entry();
                Ok(())
            }
            Mode::CoverFetchChoice { .. } => {
                self.app.skip_cover();
                Ok(())
            }
        };
        self.report(result);
    }

    fn draw_list(&mut self, ui: &mut egui::Ui) {
        ui.heading("Library");
        ui.horizontal(|ui| {
            ui.label("Filter:");
            let mut changed = false;
            egui::ComboBox::from_id_salt("filter_combo")
                .selected_text(self.app.filter.label().to_string())
                .show_ui(ui, |ui| {
                    if ui.selectable_label(self.app.filter == Filter::All, "All Entries").clicked() {
                        self.app.filter = Filter::All;
                        changed = true;
                    }
                    for system in self.app.systems() {
                        let is_selected = self.app.filter == Filter::System(system.clone());
                        if ui.selectable_label(is_selected, &system).clicked() {
                            self.app.filter = Filter::System(system);
                            changed = true;
                        }
                    }
                });
            if changed {
                self.app.clamp_selection();
                if self.app.list_state.selected().is_none() && !self.app.visible_entries().is_empty() {
                    self.app.list_state.select(Some(0));
                }
            }
        });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            let selected_id = self.app.selected_entry().map(|e| e.id);
            let rows: Vec<(usize, Uuid, String)> = self
                .app
                .visible_entries()
                .iter()
                .enumerate()
                .map(|(idx, e)| {
                    (
                        idx,
                        e.id,
                        format!("{}\n{} · {} · {:.1}h", e.title, e.system, e.status.label(), e.hours),
                    )
                })
                .collect();
            if rows.is_empty() {
                ui.weak("No entries yet — click Add below.");
            }
            for (idx, id, label) in rows {
                if ui.selectable_label(selected_id == Some(id), label).clicked() {
                    self.app.list_state.select(Some(idx));
                }
            }
        });
    }

    fn draw_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("Add").clicked() {
                self.app.begin_add_entry();
            }
            let has_selection = self.app.selected_entry().is_some();
            if ui.add_enabled(has_selection, egui::Button::new("Delete")).clicked() {
                self.app.begin_delete();
            }
            if ui.add_enabled(has_selection, egui::Button::new("Edit Title")).clicked() {
                self.app.begin_edit_title();
            }
            if ui.add_enabled(has_selection, egui::Button::new("Edit System")).clicked() {
                self.app.begin_edit_system();
            }
            if ui.add_enabled(has_selection, egui::Button::new("Release Date")).clicked() {
                self.app.begin_edit_release_date();
            }
            if ui.add_enabled(has_selection, egui::Button::new("Cycle Status")).clicked() {
                let result = self.app.cycle_status();
                self.report(result);
            }
            if ui.add_enabled(has_selection, egui::Button::new("Edit Hours")).clicked() {
                self.app.begin_edit_hours();
            }
            if ui.add_enabled(has_selection, egui::Button::new("Rating")).clicked() {
                self.app.begin_rating();
            }
            if ui.add_enabled(has_selection, egui::Button::new("Import Cover")).clicked() {
                self.app.begin_import_cover();
            }
            if ui.add_enabled(has_selection, egui::Button::new("Notes")).clicked() {
                self.app.begin_edit_notes();
            }

            let selected_id = self.app.selected_entry().map(|e| e.id);
            let session = self.app.active_session_elapsed();
            let session_label = match session {
                Some((id, elapsed)) if Some(id) == selected_id => format!("Stop Session ({})", format_duration(elapsed)),
                Some(_) => "Session Running Elsewhere".to_string(),
                None => "Start Session".to_string(),
            };
            if ui.add_enabled(has_selection, egui::Button::new(session_label)).clicked() {
                let result = self.app.toggle_session();
                self.report(result);
            }

            if ui.button("Fetch All Covers").clicked() {
                self.app.begin_fetch_all_covers();
            }
        });

        if let Some((done, succeeded, total)) = self.app.bulk_fetch_progress {
            ui.label(format!("Fetching covers: {done}/{total} ({succeeded} succeeded so far)"));
        }
        if let Some(message) = self.app.status_message.clone() {
            ui.colored_label(egui::Color32::from_rgb(120, 200, 255), message);
        }
    }

    fn draw_detail(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let Some(entry) = self.app.selected_entry().cloned() else {
            ui.label("Select an entry from the list on the left, or click Add to create one.");
            return;
        };

        ui.heading(&entry.title);
        ui.horizontal(|ui| {
            if let Some(texture) = self.cover_texture(ctx, entry.id) {
                let max_size = egui::vec2(160.0, 220.0);
                let size = texture.size_vec2();
                let scale = (max_size.x / size.x).min(max_size.y / size.y).min(1.0);
                ui.image((texture.id(), size * scale));
            } else {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(160.0, 220.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "No cover art",
                    egui::FontId::default(),
                    ui.visuals().weak_text_color(),
                );
            }

            ui.vertical(|ui| {
                ui.label(format!("System: {}", entry.system));
                ui.label(format!("Status: {}", entry.status.label()));
                let session = self
                    .app
                    .active_session_elapsed()
                    .filter(|(id, _)| *id == entry.id)
                    .map(|(_, elapsed)| elapsed);
                match session {
                    Some(elapsed) => ui.colored_label(
                        egui::Color32::YELLOW,
                        format!("Hours: {:.1}  (session: {})", entry.hours, format_duration(elapsed)),
                    ),
                    None => ui.label(format!("Hours: {:.1}", entry.hours)),
                };
                let rating = entry.rating.map(|r| stars(r.stars())).unwrap_or_else(|| "Unrated".to_string());
                ui.label(format!("Rating: {rating}"));
                let release_date = entry
                    .release_date
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                ui.label(format!("Released: {release_date}"));
            });
        });

        ui.separator();
        ui.label("Notes:");
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label(if entry.notes.is_empty() { "(none)" } else { entry.notes.as_str() });
        });
    }

    fn draw_dialog(&mut self, ctx: &egui::Context) {
        if !matches!(self.app.mode, Mode::TextInput(_)) {
            self.last_text_input_kind = None;
        }
        match self.app.mode.clone() {
            Mode::Normal => {}
            Mode::EditingNotes => self.draw_notes_dialog(ctx),
            Mode::ConfirmDelete => self.draw_confirm_delete_dialog(ctx),
            Mode::AwaitingRating => self.draw_rating_dialog(ctx),
            Mode::TextInput(kind) => self.draw_text_input_dialog(ctx, kind),
            Mode::SetupChooseSource => self.draw_setup_dialog(ctx),
            Mode::EnterApiKey { source, .. } => self.draw_api_key_dialog(ctx, source),
            Mode::CoverFetchChoice { entry_id, .. } => self.draw_cover_fetch_choice_dialog(ctx, entry_id),
        }
    }

    fn draw_notes_dialog(&mut self, ctx: &egui::Context) {
        egui::Modal::new(egui::Id::new("edit_notes_modal")).show(ctx, |ui| {
            ui.heading("Edit Notes");
            ui.add(
                egui::TextEdit::multiline(&mut self.app.input_buffer)
                    .desired_rows(10)
                    .desired_width(400.0),
            );
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    let result = self.app.commit_notes();
                    self.report(result);
                }
                if ui.button("Cancel").clicked() {
                    self.app.cancel_input();
                }
            });
        });
    }

    fn draw_confirm_delete_dialog(&mut self, ctx: &egui::Context) {
        let title = self.app.selected_entry().map(|e| e.title.clone()).unwrap_or_default();
        egui::Modal::new(egui::Id::new("confirm_delete_modal")).show(ctx, |ui| {
            ui.heading("Delete Entry");
            ui.label(format!("Delete '{title}'? This cannot be undone."));
            ui.horizontal(|ui| {
                if ui.button("Delete").clicked() {
                    let result = self.app.confirm_delete();
                    self.report(result);
                }
                if ui.button("Cancel").clicked() {
                    self.app.cancel_input();
                }
            });
        });
    }

    fn draw_rating_dialog(&mut self, ctx: &egui::Context) {
        egui::Modal::new(egui::Id::new("rating_modal")).show(ctx, |ui| {
            ui.heading("Set Rating");
            ui.horizontal(|ui| {
                for stars in 1..=5u8 {
                    if ui.button(format!("{stars} \u{2605}")).clicked() {
                        let result = self.app.set_rating(stars);
                        self.report(result);
                    }
                }
                if ui.button("Unrated").clicked() {
                    let result = self.app.clear_rating();
                    self.report(result);
                }
            });
            if ui.button("Cancel").clicked() {
                self.app.cancel_input();
            }
        });
        for (digit, key) in [
            (1u8, egui::Key::Num1),
            (2, egui::Key::Num2),
            (3, egui::Key::Num3),
            (4, egui::Key::Num4),
            (5, egui::Key::Num5),
        ] {
            if ctx.input(|i| i.key_pressed(key)) {
                let result = self.app.set_rating(digit);
                self.report(result);
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Num0)) {
            let result = self.app.clear_rating();
            self.report(result);
        }
    }

    fn draw_text_input_dialog(&mut self, ctx: &egui::Context, kind: TextInputKind) {
        egui::Modal::new(egui::Id::new("text_input_modal")).show(ctx, |ui| {
            ui.heading(kind.title());
            ui.label(kind.prompt());
            let response = ui.text_edit_singleline(&mut self.app.input_buffer);
            // Focus it whenever this is a new text-input dialog (fresh open,
            // or switching straight from one kind to another, e.g. new
            // entry's title -> system), but don't keep stealing focus back
            // every frame after that or Tab could never reach the buttons.
            if self.last_text_input_kind != Some(kind) {
                response.request_focus();
                self.last_text_input_kind = Some(kind);
            }
            let submitted = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

            ui.horizontal(|ui| {
                let ok_clicked = ui.button("OK").clicked();
                if kind == TextInputKind::ImportCoverArt
                    && ui.button("Browse...").clicked()
                    && let Some(path) = rfd::FileDialog::new().pick_file()
                {
                    self.app.input_buffer = path.display().to_string();
                    let result = self.app.commit_import_cover();
                    self.report(result);
                }
                if ui.button("Cancel").clicked() {
                    self.app.cancel_input();
                }

                if submitted || ok_clicked {
                    let result = match kind {
                        TextInputKind::NewTitle => {
                            self.app.submit_new_title();
                            Ok(())
                        }
                        TextInputKind::NewSystem => self.app.submit_new_system(),
                        TextInputKind::EditTitle => self.app.commit_title(),
                        TextInputKind::EditSystem => self.app.commit_system(),
                        TextInputKind::EditHours => self.app.commit_hours(),
                        TextInputKind::EditReleaseDate => self.app.commit_release_date(),
                        TextInputKind::ImportCoverArt => self.app.commit_import_cover(),
                    };
                    self.report(result);
                }
            });
        });
    }

    fn draw_setup_dialog(&mut self, ctx: &egui::Context) {
        egui::Modal::new(egui::Id::new("setup_modal")).show(ctx, |ui| {
            ui.heading("Welcome to gamelog!");
            ui.label("Choose a default source for cover art:");
            if ui.button("SteamGridDB (best cover/box art quality)").clicked() {
                self.app.choose_setup_source(CoverSource::SteamGridDb);
            }
            if ui.button("RAWG.io (broader metadata, banner-style art)").clicked() {
                self.app.choose_setup_source(CoverSource::Rawg);
            }
            if ui.button("Manual (you supply your own image files)").clicked() {
                self.app.choose_setup_source(CoverSource::Manual);
            }
        });
    }

    fn draw_api_key_dialog(&mut self, ctx: &egui::Context, source: CoverSource) {
        egui::Modal::new(egui::Id::new("api_key_modal")).show(ctx, |ui| {
            ui.heading(format!("Enter {} API Key", source.label()));
            let response = ui.add(egui::TextEdit::singleline(&mut self.app.input_buffer).password(true));
            if ui.memory(|mem| mem.focused().is_none()) {
                response.request_focus();
            }
            let submitted = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.horizontal(|ui| {
                let ok_clicked = ui.button("OK").clicked();
                if ui.button("Cancel").clicked() {
                    self.app.cancel_api_key_entry();
                }
                if ok_clicked || submitted {
                    let result = self.app.submit_api_key();
                    self.report(result);
                }
            });
        });
    }

    fn draw_cover_fetch_choice_dialog(&mut self, ctx: &egui::Context, entry_id: Uuid) {
        let title = self.app.library.get(entry_id).map(|e| e.title.clone()).unwrap_or_default();
        let tried = match &self.app.mode {
            Mode::CoverFetchChoice { tried, .. } => tried.clone(),
            _ => Vec::new(),
        };
        egui::Modal::new(egui::Id::new("cover_fetch_choice_modal")).show(ctx, |ui| {
            ui.heading("Cover Art Not Found");
            ui.label(format!("No cover art found for '{title}'. What would you like to do?"));
            ui.horizontal(|ui| {
                if ui.button("Try SteamGridDB").clicked() {
                    self.app.choose_fallback_source(entry_id, tried.clone(), CoverSource::SteamGridDb);
                }
                if ui.button("Try RAWG.io").clicked() {
                    self.app.choose_fallback_source(entry_id, tried.clone(), CoverSource::Rawg);
                }
                if ui.button("Import Manually").clicked() {
                    self.app.choose_manual_cover(entry_id);
                }
                if ui.button("Skip").clicked() {
                    self.app.skip_cover();
                }
            });
        });
    }
}

impl eframe::App for GamelogApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_and_invalidate();

        let ctx = ui.ctx().clone();
        if ctx.input(|i| i.viewport().close_requested()) {
            let result = self.app.quit();
            self.report(result);
        }
        self.handle_escape(&ctx);

        egui::Panel::left("entry_list")
            .resizable(true)
            .default_size(280.0)
            .show(ui, |ui| self.draw_list(ui));

        egui::Panel::bottom("toolbar").resizable(false).show(ui, |ui| self.draw_toolbar(ui));

        egui::CentralPanel::default().show(ui, |ui| self.draw_detail(&ctx, ui));

        self.draw_dialog(&ctx);

        // Keep the session timer and any in-flight fetch progress live even
        // with no user input, mirroring the TUI's fixed-interval redraw loop.
        ctx.request_repaint_after(Duration::from_millis(200));
    }
}
