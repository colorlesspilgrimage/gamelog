//! CSV import/export, for moving a library into or out of gamelog (e.g. to
//! or from a spreadsheet).
//!
//! Exported columns are `title, system, status, hours, rating, release_date,
//! last_played, notes`. Import is deliberately forgiving so hand-made or
//! spreadsheet-exported files work: only `title` and `system` are required,
//! column order and header case don't matter, unknown columns are ignored,
//! and blank cells fall back to the same defaults as a newly added entry.
//! Cover art isn't exported, since it's a path into gamelog's own data
//! directory that means nothing elsewhere.

use std::collections::HashMap;
use std::io::{Read, Write};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Local, NaiveDate, Utc};

use crate::entry::{Entry, Rating, Status};

const HEADERS: [&str; 8] = [
    "title",
    "system",
    "status",
    "hours",
    "rating",
    "release_date",
    "last_played",
    "notes",
];

/// Writes `entries` as CSV with a header row.
pub fn write_entries<W: Write>(entries: &[&Entry], writer: W) -> Result<()> {
    let mut csv = csv::Writer::from_writer(writer);
    csv.write_record(HEADERS)?;
    for entry in entries {
        csv.write_record([
            entry.title.as_str(),
            entry.system.as_str(),
            status_name(entry.status),
            // f32's Display is the shortest string that round-trips exactly.
            &entry.hours.to_string(),
            &entry
                .rating
                .map(|r| r.stars().to_string())
                .unwrap_or_default(),
            &entry
                .release_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default(),
            &entry
                .last_played
                .map(|t| t.to_rfc3339())
                .unwrap_or_default(),
            entry.notes.as_str(),
        ])?;
    }
    csv.flush()?;
    Ok(())
}

/// The outcome of parsing a CSV file: every row that parsed cleanly, plus a
/// `(line number, reason)` for each row that didn't. A bad row never aborts
/// the whole import.
#[derive(Debug, Default)]
pub struct ParsedCsv {
    pub entries: Vec<Entry>,
    pub bad_rows: Vec<(u64, String)>,
}

/// Parses CSV into new entries (each with a fresh id). Fails outright only
/// if the file can't be read as CSV at all or lacks a `title` or `system`
/// column.
pub fn read_entries<R: Read>(reader: R) -> Result<ParsedCsv> {
    let mut csv = csv::ReaderBuilder::new().flexible(true).from_reader(reader);
    let columns: HashMap<String, usize> = csv
        .headers()
        .context("reading the header row")?
        .iter()
        .enumerate()
        // Spreadsheet apps often prefix the first header with a UTF-8 BOM.
        .map(|(i, h)| (h.trim_start_matches('\u{feff}').trim().to_lowercase(), i))
        .collect();
    let column = |names: &[&str]| names.iter().find_map(|n| columns.get(*n).copied());
    let Some(title_col) = column(&["title", "name"]) else {
        bail!("no \"title\" column in the header row");
    };
    let Some(system_col) = column(&["system", "platform"]) else {
        bail!("no \"system\" column in the header row");
    };
    let status_col = column(&["status"]);
    let hours_col = column(&["hours", "hours_played"]);
    let rating_col = column(&["rating"]);
    let release_col = column(&["release_date", "released"]);
    let last_played_col = column(&["last_played"]);
    let notes_col = column(&["notes"]);

    let mut parsed = ParsedCsv::default();
    for record in csv.records() {
        let record = record.context("reading a row")?;
        let line = record.position().map_or(0, |p| p.line());
        let cell = |col: Option<usize>| col.and_then(|c| record.get(c)).unwrap_or("").trim();

        let row = (|| -> Result<Entry, String> {
            let title = cell(Some(title_col));
            let system = cell(Some(system_col));
            if title.is_empty() {
                return Err("title is empty".to_string());
            }
            if system.is_empty() {
                return Err("system is empty".to_string());
            }
            let mut entry = Entry::new(title, system);
            entry.status = parse_status(cell(status_col))?;
            entry.hours = parse_hours(cell(hours_col))?;
            entry.rating = parse_rating(cell(rating_col))?;
            entry.release_date = parse_release_date(cell(release_col))?;
            entry.last_played = parse_last_played(cell(last_played_col))?;
            // Notes keep their original whitespace, unlike the other cells.
            entry.notes = notes_col
                .and_then(|c| record.get(c))
                .unwrap_or("")
                .to_string();
            Ok(entry)
        })();
        match row {
            Ok(entry) => parsed.entries.push(entry),
            Err(reason) => parsed.bad_rows.push((line, reason)),
        }
    }
    Ok(parsed)
}

fn status_name(status: Status) -> &'static str {
    match status {
        Status::WantToPlay => "want_to_play",
        Status::Playing => "playing",
        Status::Played => "played",
    }
}

/// Accepts gamelog's own names plus common spellings from other trackers,
/// ignoring case, spaces, and punctuation ("Want to Play", "want_to_play").
fn parse_status(raw: &str) -> Result<Status, String> {
    let normalized: String = raw
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_lowercase();
    match normalized.as_str() {
        "" | "wanttoplay" | "want" | "backlog" | "planned" => Ok(Status::WantToPlay),
        "playing" | "inprogress" | "started" => Ok(Status::Playing),
        "played" | "finished" | "completed" | "beaten" | "done" => Ok(Status::Played),
        _ => Err(format!("unrecognized status \"{raw}\"")),
    }
}

fn parse_hours(raw: &str) -> Result<f32, String> {
    if raw.is_empty() {
        return Ok(0.0);
    }
    match raw.parse::<f32>() {
        Ok(hours) if hours.is_finite() && hours >= 0.0 => Ok(hours),
        _ => Err(format!("invalid hours \"{raw}\" (expected a number >= 0)")),
    }
}

fn parse_rating(raw: &str) -> Result<Option<Rating>, String> {
    if raw.is_empty() {
        return Ok(None);
    }
    match raw.parse::<u8>() {
        Ok(stars) if stars <= Rating::MAX => Ok(Some(Rating::new(stars))),
        _ => Err(format!("invalid rating \"{raw}\" (expected 0-5)")),
    }
}

fn parse_release_date(raw: &str) -> Result<Option<NaiveDate>, String> {
    if raw.is_empty() {
        return Ok(None);
    }
    NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .map(Some)
        .map_err(|_| format!("invalid release date \"{raw}\" (expected YYYY-MM-DD)"))
}

/// Accepts a full RFC 3339 timestamp (what export writes) or a bare
/// YYYY-MM-DD date, taken as local midnight so it displays as that date.
fn parse_last_played(raw: &str) -> Result<Option<DateTime<Utc>>, String> {
    if raw.is_empty() {
        return Ok(None);
    }
    if let Ok(timestamp) = DateTime::parse_from_rfc3339(raw) {
        return Ok(Some(timestamp.with_timezone(&Utc)));
    }
    NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0)?.and_local_timezone(Local).earliest())
        .map(|t| Some(t.with_timezone(&Utc)))
        .ok_or_else(|| {
            format!("invalid last played \"{raw}\" (expected YYYY-MM-DD or an RFC 3339 timestamp)")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(csv: &str) -> ParsedCsv {
        read_entries(csv.as_bytes()).unwrap()
    }

    #[test]
    fn export_then_import_round_trips_every_field() {
        let mut entry = Entry::new("Hades, \"Director's\" Cut", "PC");
        entry.status = Status::Played;
        entry.hours = 61.333_336;
        entry.rating = Some(Rating::new(5));
        entry.release_date = NaiveDate::from_ymd_opt(2020, 9, 17);
        entry.last_played = DateTime::from_timestamp(1_758_000_000, 123_000_000);
        entry.notes = "Line one, with a comma\nLine two".to_string();

        let mut buffer = Vec::new();
        write_entries(&[&entry], &mut buffer).unwrap();
        let parsed = read_entries(buffer.as_slice()).unwrap();

        assert!(parsed.bad_rows.is_empty(), "{:?}", parsed.bad_rows);
        let back = &parsed.entries[0];
        assert_eq!(back.title, entry.title);
        assert_eq!(back.system, entry.system);
        assert_eq!(back.status, entry.status);
        assert_eq!(back.hours, entry.hours);
        assert_eq!(back.rating, entry.rating);
        assert_eq!(back.release_date, entry.release_date);
        assert_eq!(back.last_played, entry.last_played);
        assert_eq!(back.notes, entry.notes);
        assert_ne!(back.id, entry.id, "imported entries get fresh ids");
    }

    #[test]
    fn import_needs_only_title_and_system_in_any_order_and_case() {
        let parsed = parse("\u{feff}Platform,Extra,NAME\nSwitch,ignored,Celeste\n");
        assert!(parsed.bad_rows.is_empty());
        let entry = &parsed.entries[0];
        assert_eq!(
            (entry.title.as_str(), entry.system.as_str()),
            ("Celeste", "Switch")
        );
        assert_eq!(entry.status, Status::WantToPlay);
        assert_eq!(entry.hours, 0.0);
        assert_eq!(entry.rating, None);
    }

    #[test]
    fn missing_required_column_fails_the_whole_import() {
        assert!(read_entries("title,hours\nHades,3\n".as_bytes()).is_err());
    }

    #[test]
    fn bad_rows_are_reported_by_line_without_stopping_the_import() {
        let parsed = parse(
            "title,system,status,hours,rating\n\
             Good,PC,Completed,4.5,4\n\
             ,PC,,,\n\
             BadRating,PC,,,7\n\
             BadStatus,PC,abandoned,,\n\
             BadHours,PC,,-1,\n",
        );
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].status, Status::Played);
        let lines: Vec<u64> = parsed.bad_rows.iter().map(|(line, _)| *line).collect();
        assert_eq!(lines, [3, 4, 5, 6]);
    }

    #[test]
    fn last_played_accepts_a_bare_date() {
        let parsed = parse("title,system,last_played\nHades,PC,2025-06-01\n");
        let local = parsed.entries[0].last_played.unwrap().with_timezone(&Local);
        assert_eq!(
            local.date_naive(),
            NaiveDate::from_ymd_opt(2025, 6, 1).unwrap()
        );
    }

    #[test]
    fn status_accepts_common_spellings() {
        assert_eq!(parse_status("Want to Play"), Ok(Status::WantToPlay));
        assert_eq!(parse_status("want_to_play"), Ok(Status::WantToPlay));
        assert_eq!(parse_status("In Progress"), Ok(Status::Playing));
        assert_eq!(parse_status("Beaten"), Ok(Status::Played));
        assert!(parse_status("abandoned").is_err());
    }
}
