use std::thread;
use std::time::Duration;

use serde_json::Value;

use crate::settings::CoverSource;

/// Distinguishes a rate-limit response (worth retrying after a delay) from
/// any other failure (bad key, network error, unexpected response shape).
#[derive(Debug)]
enum FetchError {
    RateLimited,
    Other(anyhow::Error),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::RateLimited => write!(f, "rate limited"),
            FetchError::Other(err) => write!(f, "{err}"),
        }
    }
}

fn map_fetch_err(err: ureq::Error) -> FetchError {
    if matches!(err, ureq::Error::StatusCode(429)) {
        FetchError::RateLimited
    } else {
        FetchError::Other(err.into())
    }
}

type FetchOutcome = Result<Option<(Vec<u8>, String)>, FetchError>;

const MAX_ATTEMPTS: u32 = 3;
const INITIAL_BACKOFF: Duration = Duration::from_secs(2);

/// Attempts to find and download cover art for `title` from `source`,
/// automatically retrying with exponential backoff if the API responds
/// with a rate-limit (429) status, up to `MAX_ATTEMPTS` tries total.
///
/// Returns `Ok(None)` when the source has no matching results (not an
/// error, just an empty search); returns `Err` for any other failure
/// (bad key, timeout, unexpected response shape, or persistent rate
/// limiting), stringified so it can cross a thread boundary cheaply.
pub fn fetch_cover_with_retry(
    source: CoverSource,
    api_key: &str,
    title: &str,
) -> Result<Option<(Vec<u8>, String)>, String> {
    let mut backoff = INITIAL_BACKOFF;
    for attempt in 1..=MAX_ATTEMPTS {
        match fetch_cover(source, api_key, title) {
            Ok(result) => return Ok(result),
            Err(FetchError::RateLimited) if attempt < MAX_ATTEMPTS => {
                thread::sleep(backoff);
                backoff *= 2;
            }
            Err(err) => return Err(err.to_string()),
        }
    }
    Err("rate limited after retrying".to_string())
}

fn fetch_cover(source: CoverSource, api_key: &str, title: &str) -> FetchOutcome {
    match source {
        CoverSource::SteamGridDb => fetch_steamgriddb(api_key, title),
        CoverSource::Rawg => fetch_rawg(api_key, title),
        CoverSource::Manual => Err(FetchError::Other(anyhow::anyhow!(
            "Manual is not a fetchable cover art source"
        ))),
    }
}

fn fetch_steamgriddb(api_key: &str, title: &str) -> FetchOutcome {
    let search_url = format!(
        "https://www.steamgriddb.com/api/v2/search/autocomplete/{}",
        urlencoding::encode(title)
    );
    let mut response = ureq::get(&search_url)
        .header("Authorization", format!("Bearer {api_key}"))
        .call()
        .map_err(map_fetch_err)?;
    let json: Value = response.body_mut().read_json().map_err(map_fetch_err)?;
    let Some(game_id) = json["data"].get(0).and_then(|g| g["id"].as_u64()) else {
        return Ok(None);
    };

    let grids_url = format!("https://www.steamgriddb.com/api/v2/grids/game/{game_id}");
    let mut response = ureq::get(&grids_url)
        .header("Authorization", format!("Bearer {api_key}"))
        .call()
        .map_err(map_fetch_err)?;
    let json: Value = response.body_mut().read_json().map_err(map_fetch_err)?;
    let Some(image_url) = json["data"].get(0).and_then(|g| g["url"].as_str()) else {
        return Ok(None);
    };

    download_image(image_url)
}

fn fetch_rawg(api_key: &str, title: &str) -> FetchOutcome {
    let search_url = format!(
        "https://api.rawg.io/api/games?search={}&key={}&page_size=1",
        urlencoding::encode(title),
        urlencoding::encode(api_key)
    );
    let mut response = ureq::get(&search_url).call().map_err(map_fetch_err)?;
    let json: Value = response.body_mut().read_json().map_err(map_fetch_err)?;
    let Some(image_url) = json["results"]
        .get(0)
        .and_then(|g| g["background_image"].as_str())
    else {
        return Ok(None);
    };

    download_image(image_url)
}

fn download_image(url: &str) -> FetchOutcome {
    let mut response = ureq::get(url).call().map_err(map_fetch_err)?;
    let extension = if url.contains(".png") { "png" } else { "jpg" };
    let bytes = response.body_mut().read_to_vec().map_err(map_fetch_err)?;
    Ok(Some((bytes, extension.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_429_as_rate_limited() {
        let err = map_fetch_err(ureq::Error::StatusCode(429));
        assert!(matches!(err, FetchError::RateLimited));
    }

    #[test]
    fn classifies_other_status_codes_as_other() {
        let err = map_fetch_err(ureq::Error::StatusCode(401));
        assert!(matches!(err, FetchError::Other(_)));
    }
}
