use std::fs::read_dir;
use std::path::{Path, PathBuf};
use listui_lib::models::Track;
use  listui_lib::{api::ApiClient, db::Dao};
use regex::Regex;
use std::env;

// On success, returns the id of the new playlist stored in the DB.
pub fn get_youtube_playlist(database_path: &Path, url: &str) -> Result<Option<i32>, Box<dyn std::error::Error>> {

    let re = Regex::new(r"^https?://(?:w{3}.)?(?:(?:youtube\.com)|(?:youtu\.be))/.+\?(?:.+&)*list=(PL.+?)(?:&|$)").expect("Failed to compile regex.");
    
    let re_match = re.captures(url)
        .and_then(|c| c.get(1));

    if let Some(id) = re_match {

        let dto = Dao::new(database_path)?;

        let yt_api_key = env::var("YT_API_KEY");
        let client = match yt_api_key {
            Ok(key) => {
                println!("Fetching videos from YouTube api...");
                ApiClient::from_youtube( key)
            },
            Err(_) => {
                println!("Fetching videos from Invidious api. This can take up to a few minutes.");
                ApiClient::from_invidious()
            }
        };

        let (playlist, videos) = client.fetch_playlist(id.as_str())?;
        let n_videos = videos.len();

        let pl = dto.save_playlist(playlist)?;
        dto.save_tracks(videos, pl.id)?;

        println!("Succesfully fetched {}, containing {n_videos} songs.", pl.title);
        Ok(Some(pl.id))
    }
    else { Ok(None) }
}

// Returns a list of the tracks inside a local directory. Only works with mp3 files currently.
pub fn get_local_playlist(path: &PathBuf) -> Option<Vec<Track>> {

    if path.is_dir() {
        
        let tracks = read_dir(path).ok()?
            .flatten()
            .filter_map(|entry| {
                let filename = entry.file_name();
                let filename = filename.to_string_lossy();
                if filename.ends_with(".mp3") {
                    Some(Track{
                        id: 0,
                        title: entry.path().with_extension("").file_name().unwrap().to_string_lossy().to_string(),
                        yt_id: None,
                        playlist_id: None,
                    })
                }
                else { None }
            })
            .collect();

        Some(tracks)
    }

    else { None }
}

pub fn time_str(s1: i32, s2: i32) -> String {

    let (m1, s1) = (s1 / 60, s1 % 60);
    let (h1, m1) = (m1 / 60, m1 % 60);

    let (m2, s2) = (s2 / 60, s2 % 60);
    let (h2, m2) = (m2 / 60, m2 % 60);

    if h2 == 0 { format!("{:02}:{:02} / {:02}:{:02}", m1, s1,  m2, s2) }
    else { format!("{:02}:{:02}:{:02} / {:02}:{:02}:{:02}", h1, m1, s1, h2, m2, s2) }
}