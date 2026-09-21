use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::BaseDirs;

/// The hidden `.gamelog` directory in the user's home directory. All
/// application data and config lives here rather than in the
/// platform-visible XDG/Library directories, so it doesn't clutter a normal
/// directory listing.
pub fn gamelog_dir() -> Result<PathBuf> {
    let base_dirs = BaseDirs::new().context("could not determine the home directory")?;
    Ok(base_dirs.home_dir().join(".gamelog"))
}
