use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entry::Entry;

const CURRENT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct LibraryFile {
    version: u32,
    entries: Vec<Entry>,
}

pub struct Library {
    entries: Vec<Entry>,
    data_dir: PathBuf,
}

impl Library {
    fn data_dir() -> Result<PathBuf> {
        let dirs = ProjectDirs::from("", "", "gamelog")
            .context("could not determine a data directory for this platform")?;
        Ok(dirs.data_dir().to_path_buf())
    }

    fn entries_path(data_dir: &Path) -> PathBuf {
        data_dir.join("entries.json")
    }

    fn covers_dir(data_dir: &Path) -> PathBuf {
        data_dir.join("covers")
    }

    /// Loads the library from disk, creating the data directory and an empty
    /// library if this is the first run.
    pub fn load() -> Result<Self> {
        let data_dir = Self::data_dir()?;
        fs::create_dir_all(&data_dir)
            .with_context(|| format!("creating data directory at {}", data_dir.display()))?;
        fs::create_dir_all(Self::covers_dir(&data_dir)).context("creating covers directory")?;

        let entries_path = Self::entries_path(&data_dir);
        let entries = if entries_path.exists() {
            let raw = fs::read_to_string(&entries_path)
                .with_context(|| format!("reading {}", entries_path.display()))?;
            let file: LibraryFile = serde_json::from_str(&raw)
                .with_context(|| format!("parsing {}", entries_path.display()))?;
            file.entries
        } else {
            Vec::new()
        };

        Ok(Self { entries, data_dir })
    }

    /// Writes the library to disk atomically: the new contents are written to
    /// a temp file first, then renamed over the real file, so a crash
    /// mid-write can never leave `entries.json` truncated or corrupt.
    pub fn save(&self) -> Result<()> {
        let entries_path = Self::entries_path(&self.data_dir);
        let file = LibraryFile {
            version: CURRENT_VERSION,
            entries: self.entries.clone(),
        };
        let json = serde_json::to_string_pretty(&file)?;

        let tmp_path = entries_path.with_extension("json.tmp");
        {
            let mut tmp_file = fs::File::create(&tmp_path)
                .with_context(|| format!("creating {}", tmp_path.display()))?;
            tmp_file.write_all(json.as_bytes())?;
            tmp_file.sync_all()?;
        }
        fs::rename(&tmp_path, &entries_path)
            .with_context(|| format!("renaming into place: {}", entries_path.display()))?;

        Ok(())
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn get(&self, id: Uuid) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn get_mut(&mut self, id: Uuid) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|e| e.id == id)
    }

    pub fn add(&mut self, entry: Entry) {
        self.entries.push(entry);
    }

    pub fn remove(&mut self, id: Uuid) -> Option<Entry> {
        let pos = self.entries.iter().position(|e| e.id == id)?;
        Some(self.entries.remove(pos))
    }

    /// Copies an image file into the app's data directory as `entry_id`'s
    /// cover art, returning a path relative to the data directory to store
    /// on `Entry::cover_art`.
    pub fn import_cover_art(&self, entry_id: Uuid, source: &Path) -> Result<PathBuf> {
        let ext = source
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png");
        let file_name = format!("{entry_id}.{ext}");
        let dest = Self::covers_dir(&self.data_dir).join(&file_name);
        fs::copy(source, &dest)
            .with_context(|| format!("copying cover art from {}", source.display()))?;
        Ok(PathBuf::from("covers").join(file_name))
    }

    /// Resolves a stored (relative) cover art path to an absolute path on disk.
    pub fn resolve_cover_art(&self, relative: &Path) -> PathBuf {
        self.data_dir.join(relative)
    }

    #[cfg(test)]
    fn at(data_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&data_dir)?;
        fs::create_dir_all(Self::covers_dir(&data_dir))?;
        Ok(Self {
            entries: Vec::new(),
            data_dir,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::Status;

    fn temp_dir() -> PathBuf {
        std::env::temp_dir().join(format!("gamelog-test-{}", Uuid::new_v4()))
    }

    #[test]
    fn save_and_reload_round_trips_entries() {
        let dir = temp_dir();
        let mut lib = Library::at(dir.clone()).unwrap();

        let mut entry = Entry::new("Hades", "PC");
        entry.status = Status::Played;
        entry.hours = 42.5;
        lib.add(entry.clone());
        lib.save().unwrap();

        // Reload by pointing a fresh Library at the same directory.
        let raw = fs::read_to_string(dir.join("entries.json")).unwrap();
        let file: LibraryFile = serde_json::from_str(&raw).unwrap();

        assert_eq!(file.entries.len(), 1);
        assert_eq!(file.entries[0].title, "Hades");
        assert_eq!(file.entries[0].hours, 42.5);

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn import_cover_art_copies_file_and_returns_relative_path() {
        let dir = temp_dir();
        let lib = Library::at(dir.clone()).unwrap();

        let source = dir.join("source.png");
        fs::write(&source, b"fake png bytes").unwrap();

        let entry_id = Uuid::new_v4();
        let relative = lib.import_cover_art(entry_id, &source).unwrap();

        assert_eq!(relative, PathBuf::from("covers").join(format!("{entry_id}.png")));
        let absolute = lib.resolve_cover_art(&relative);
        assert!(absolute.exists());
        assert_eq!(fs::read(absolute).unwrap(), b"fake png bytes");

        fs::remove_dir_all(dir).ok();
    }
}
