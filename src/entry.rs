use std::cmp::Ordering;
use std::path::PathBuf;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    WantToPlay,
    Playing,
    Played,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::WantToPlay => "Want to Play",
            Status::Playing => "Playing",
            Status::Played => "Played",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Status::WantToPlay => Status::Playing,
            Status::Playing => Status::Played,
            Status::Played => Status::WantToPlay,
        }
    }
}

/// A star rating from 0 to 5, inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Rating(u8);

impl Rating {
    pub const MIN: u8 = 0;
    pub const MAX: u8 = 5;

    pub fn new(stars: u8) -> Self {
        Self(stars.min(Self::MAX))
    }

    pub fn stars(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: Uuid,
    pub title: String,
    pub release_date: Option<NaiveDate>,
    pub system: String,
    pub status: Status,
    pub hours: f32,
    pub rating: Option<Rating>,
    /// Path to cover art, relative to the app's data directory.
    pub cover_art: Option<PathBuf>,
    /// Freeform, persistent notes about this entry.
    #[serde(default)]
    pub notes: String,
    /// When a play session for this entry last ended. `None` for entries
    /// never timed with a session (including all pre-existing entries).
    #[serde(default)]
    pub last_played: Option<DateTime<Utc>>,
}

impl Entry {
    pub fn new(title: impl Into<String>, system: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            release_date: None,
            system: system.into(),
            status: Status::WantToPlay,
            hours: 0.0,
            rating: None,
            cover_art: None,
            notes: String::new(),
            last_played: None,
        }
    }
}

impl Entry {
    /// Whether this entry matches a title search query: a case-insensitive
    /// substring match. An empty (or all-whitespace) query matches everything.
    pub fn matches_query(&self, query: &str) -> bool {
        let query = query.trim();
        query.is_empty() || self.title.to_lowercase().contains(&query.to_lowercase())
    }
}

/// What the entry list is ordered by. Each key has a natural direction
/// (A–Z for titles, most/highest/newest first for numbers and dates), which
/// `reversed` flips. Entries missing the sorted-on value (unrated, unknown
/// release date) always sink to the bottom regardless of direction, and
/// ties are always broken by title A–Z so the order is stable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortKey {
    #[default]
    Title,
    Hours,
    Rating,
    ReleaseDate,
    LastPlayed,
    Status,
}

impl SortKey {
    /// Every sort key, in cycling order, for pickers that list them all.
    pub const ALL: [SortKey; 6] = [
        SortKey::Title,
        SortKey::Hours,
        SortKey::Rating,
        SortKey::ReleaseDate,
        SortKey::LastPlayed,
        SortKey::Status,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SortKey::Title => "Title",
            SortKey::Hours => "Hours",
            SortKey::Rating => "Rating",
            SortKey::ReleaseDate => "Release Date",
            SortKey::LastPlayed => "Last Played",
            SortKey::Status => "Status",
        }
    }

    pub fn next(self) -> Self {
        match self {
            SortKey::Title => SortKey::Hours,
            SortKey::Hours => SortKey::Rating,
            SortKey::Rating => SortKey::ReleaseDate,
            SortKey::ReleaseDate => SortKey::LastPlayed,
            SortKey::LastPlayed => SortKey::Status,
            SortKey::Status => SortKey::Title,
        }
    }

    pub fn compare(self, a: &Entry, b: &Entry, reversed: bool) -> Ordering {
        let (a_missing, b_missing) = (self.is_missing(a), self.is_missing(b));
        if a_missing != b_missing {
            return a_missing.cmp(&b_missing);
        }
        let primary = match self {
            SortKey::Title => title_order(a, b),
            SortKey::Hours => b.hours.total_cmp(&a.hours),
            SortKey::Rating => b.rating.cmp(&a.rating),
            SortKey::ReleaseDate => b.release_date.cmp(&a.release_date),
            SortKey::LastPlayed => b.last_played.cmp(&a.last_played),
            SortKey::Status => status_rank(a.status).cmp(&status_rank(b.status)),
        };
        let primary = if reversed { primary.reverse() } else { primary };
        primary.then_with(|| title_order(a, b))
    }

    /// Whether `entry` lacks a value for this key, and so sorts last.
    fn is_missing(self, entry: &Entry) -> bool {
        match self {
            SortKey::Rating => entry.rating.is_none(),
            SortKey::ReleaseDate => entry.release_date.is_none(),
            SortKey::LastPlayed => entry.last_played.is_none(),
            SortKey::Title | SortKey::Hours | SortKey::Status => false,
        }
    }
}

/// Case-insensitive title order, falling back to a case-sensitive
/// comparison so titles differing only in case still order deterministically.
fn title_order(a: &Entry, b: &Entry) -> Ordering {
    a.title
        .to_lowercase()
        .cmp(&b.title.to_lowercase())
        .then_with(|| a.title.cmp(&b.title))
}

/// Status sort order: what you're currently playing first, then the
/// backlog, then finished games.
fn status_rank(status: Status) -> u8 {
    match status {
        Status::Playing => 0,
        Status::WantToPlay => 1,
        Status::Played => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(title: &str) -> Entry {
        Entry::new(title, "PC")
    }

    fn sorted_titles(entries: &mut [Entry], key: SortKey, reversed: bool) -> Vec<String> {
        entries.sort_by(|a, b| key.compare(a, b, reversed));
        entries.iter().map(|e| e.title.clone()).collect()
    }

    #[test]
    fn title_search_is_case_insensitive_substring() {
        let e = entry("The Legend of Zelda");
        assert!(e.matches_query("zelda"));
        assert!(e.matches_query("LEGEND of"));
        assert!(e.matches_query("  "));
        assert!(!e.matches_query("mario"));
    }

    #[test]
    fn title_sort_ignores_case() {
        let mut entries = vec![entry("celeste"), entry("Bastion"), entry("Hades")];
        assert_eq!(
            sorted_titles(&mut entries, SortKey::Title, false),
            ["Bastion", "celeste", "Hades"]
        );
        assert_eq!(
            sorted_titles(&mut entries, SortKey::Title, true),
            ["Hades", "celeste", "Bastion"]
        );
    }

    #[test]
    fn hours_sort_is_most_first_with_title_tiebreak() {
        let mut a = entry("B");
        a.hours = 10.0;
        let mut b = entry("A");
        b.hours = 10.0;
        let mut c = entry("C");
        c.hours = 50.0;
        let mut entries = vec![a, b, c];
        assert_eq!(
            sorted_titles(&mut entries, SortKey::Hours, false),
            ["C", "A", "B"]
        );
        // Reversing flips the primary order but the tiebreak stays A-Z.
        assert_eq!(
            sorted_titles(&mut entries, SortKey::Hours, true),
            ["A", "B", "C"]
        );
    }

    #[test]
    fn unrated_entries_sort_last_in_either_direction() {
        let mut low = entry("Low");
        low.rating = Some(Rating::new(2));
        let mut high = entry("High");
        high.rating = Some(Rating::new(5));
        let unrated = entry("Unrated");
        let mut entries = vec![unrated, low, high];
        assert_eq!(
            sorted_titles(&mut entries, SortKey::Rating, false),
            ["High", "Low", "Unrated"]
        );
        assert_eq!(
            sorted_titles(&mut entries, SortKey::Rating, true),
            ["Low", "High", "Unrated"]
        );
    }

    #[test]
    fn release_date_sort_is_newest_first_unknown_last() {
        let mut old = entry("Old");
        old.release_date = NaiveDate::from_ymd_opt(1998, 11, 21);
        let mut new = entry("New");
        new.release_date = NaiveDate::from_ymd_opt(2023, 5, 12);
        let unknown = entry("Unknown");
        let mut entries = vec![unknown, old, new];
        assert_eq!(
            sorted_titles(&mut entries, SortKey::ReleaseDate, false),
            ["New", "Old", "Unknown"]
        );
    }

    #[test]
    fn status_sort_puts_playing_first() {
        let mut played = entry("Played");
        played.status = Status::Played;
        let mut playing = entry("Playing");
        playing.status = Status::Playing;
        let backlog = entry("Backlog");
        let mut entries = vec![played, backlog, playing];
        assert_eq!(
            sorted_titles(&mut entries, SortKey::Status, false),
            ["Playing", "Backlog", "Played"]
        );
    }

    #[test]
    fn last_played_sort_is_most_recent_first_never_played_last() {
        let mut earlier = entry("Earlier");
        earlier.last_played = DateTime::from_timestamp(1_700_000_000, 0);
        let mut recent = entry("Recent");
        recent.last_played = DateTime::from_timestamp(1_750_000_000, 0);
        let never = entry("Never");
        let mut entries = vec![never, earlier, recent];
        assert_eq!(
            sorted_titles(&mut entries, SortKey::LastPlayed, false),
            ["Recent", "Earlier", "Never"]
        );
        assert_eq!(
            sorted_titles(&mut entries, SortKey::LastPlayed, true),
            ["Earlier", "Recent", "Never"]
        );
    }

    #[test]
    fn entries_saved_before_last_played_existed_still_load() {
        let json = r#"{"id":"5ad71341-6fc2-48ba-9041-23ca4bff0af4","title":"Hades",
            "release_date":null,"system":"PC","status":"Played","hours":1.0,
            "rating":null,"cover_art":null,"notes":""}"#;
        let e: Entry = serde_json::from_str(json).unwrap();
        assert_eq!(e.last_played, None);
    }
}
