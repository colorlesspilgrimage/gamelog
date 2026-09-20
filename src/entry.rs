use std::path::PathBuf;

use chrono::NaiveDate;
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
        }
    }
}
