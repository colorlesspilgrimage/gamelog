use anyhow::{bail, Result};
use serde_json::Value;

use crate::settings::CoverSource;

/// Attempts to find and download cover art for `title` from `source`.
///
/// Returns `Ok(None)` when the source has no matching results (not an
/// error, just an empty search); returns `Err` for network or API
/// failures (bad key, timeout, unexpected response shape, etc).
pub fn fetch_cover(source: CoverSource, api_key: &str, title: &str) -> Result<Option<(Vec<u8>, String)>> {
    match source {
        CoverSource::SteamGridDb => fetch_steamgriddb(api_key, title),
        CoverSource::Rawg => fetch_rawg(api_key, title),
        CoverSource::Manual => bail!("Manual is not a fetchable cover art source"),
    }
}

fn fetch_steamgriddb(api_key: &str, title: &str) -> Result<Option<(Vec<u8>, String)>> {
    let search_url = format!(
        "https://www.steamgriddb.com/api/v2/search/autocomplete/{}",
        urlencoding::encode(title)
    );
    let mut response = ureq::get(&search_url)
        .header("Authorization", format!("Bearer {api_key}"))
        .call()?;
    let json: Value = response.body_mut().read_json()?;
    let Some(game_id) = json["data"].get(0).and_then(|g| g["id"].as_u64()) else {
        return Ok(None);
    };

    let grids_url = format!("https://www.steamgriddb.com/api/v2/grids/game/{game_id}");
    let mut response = ureq::get(&grids_url)
        .header("Authorization", format!("Bearer {api_key}"))
        .call()?;
    let json: Value = response.body_mut().read_json()?;
    let Some(image_url) = json["data"].get(0).and_then(|g| g["url"].as_str()) else {
        return Ok(None);
    };

    download_image(image_url)
}

fn fetch_rawg(api_key: &str, title: &str) -> Result<Option<(Vec<u8>, String)>> {
    let search_url = format!(
        "https://api.rawg.io/api/games?search={}&key={}&page_size=1",
        urlencoding::encode(title),
        urlencoding::encode(api_key)
    );
    let mut response = ureq::get(&search_url).call()?;
    let json: Value = response.body_mut().read_json()?;
    let Some(image_url) = json["results"]
        .get(0)
        .and_then(|g| g["background_image"].as_str())
    else {
        return Ok(None);
    };

    download_image(image_url)
}

fn download_image(url: &str) -> Result<Option<(Vec<u8>, String)>> {
    let mut response = ureq::get(url).call()?;
    let extension = if url.contains(".png") { "png" } else { "jpg" };
    let bytes = response.body_mut().read_to_vec()?;
    Ok(Some((bytes, extension.to_string())))
}
