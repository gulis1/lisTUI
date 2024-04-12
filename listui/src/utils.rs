use std::fs::{create_dir_all, read_dir};
use std::path::{Path, PathBuf};
use listui_lib::models::{Track, NewVideo, NewPlaylist};
use listui_lib::api::{ApiClient, ApiError, ApiProgressCallback};
use regex::Regex;
use std::env;
use std::process::{Command, Stdio};

#[derive(Debug)]
pub enum Message {
    SongFinished,
    NewPlaylist(Result<(NewPlaylist, Vec<NewVideo>), ApiError>),
    PlaylistUpdate(Result<(i32, Vec<NewVideo>), ApiError>),
    DownloadProgress(String)
}

#[derive(Debug)]
pub struct ProbingError {}
impl std::error::Error for ProbingError {}
impl std::fmt::Display for ProbingError {
    
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Please install ffmpeg and yt-dlp fist.")
    }
}

pub fn parse_playlist_url(url: &str) -> Option<String> {
    
    let re = Regex::new(r"^https?://(?:w{3}.)?(?:(?:youtube\.com)|(?:youtu\.be))/.+\?(?:.+&)*list=(PL.+?)(?:&|$)").expect("Failed to compile regex.");
    Some(String::from(re.captures(url)  .and_then(|c| c.get(1))?.as_str()))
}

// On success, returns the id of the new playlist stored in the DB.
pub async fn get_youtube_playlist(playlist_id: &str, callback: Option<ApiProgressCallback>) -> Result<(NewPlaylist, Vec<NewVideo>), ApiError> {

    let yt_api_key = env::var("YT_API_KEY");
    let client = match yt_api_key {
        Ok(key) => {
            // if print_messages { println!("Fetching videos from YouTube api...") };
            ApiClient::from_youtube(key, callback)
        },
        Err(_) => {
            // if print_messages { println!("Fetching videos from Invidious api. This can take up to a few minutes.") };
            ApiClient::from_invidious(callback)
        }
    };
    let (playlist, videos) = client.fetch_playlist(playlist_id).await?;
    // if print_messages { println!("Succesfully fetched {}, containing {} songs.", playlist.title, videos.len()); }

    Ok((playlist, videos))
}

// Returns a list of the tracks inside a local directory. Only works with mp3 files currently.
pub fn get_local_playlist(path: &Path) -> Option<Vec<Track>> {

    if path.is_dir() {
        
        let path = path.canonicalize().ok()?;
        let tracks = read_dir(path).ok()?
            .flatten()
            .enumerate()
            .filter_map(|(ind, entry)| {
                let filename = entry.file_name();
                let filename = filename.to_string_lossy();
                if filename.ends_with(".mp3") {
                    Some(Track{
                        id: ind as i32,
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

pub fn time_str(s1: i32, s2: i32, paused: bool) -> String {

    let separator = if paused {"▮▮"} else {"▶"};

    let (m1, s1) = (s1 / 60, s1 % 60);
    let (h1, m1) = (m1 / 60, m1 % 60);

    let (m2, s2) = (s2 / 60, s2 % 60);
    let (h2, m2) = (m2 / 60, m2 % 60);

    if h2 == 0 { format!("{:02}:{:02} {separator} {:02}:{:02}", m1, s1,  m2, s2) }
    else { format!("{:02}:{:02}:{:02} {separator} {:02}:{:02}:{:02}", h1, m1, s1, h2, m2, s2) }
}

pub fn probe_ytdlp() -> bool {

    let child = Command::new("yt-dlp")
        .arg("--help")
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .spawn();

    child.is_ok()
}

pub fn probe_ffmpeg() -> bool {

    let child = Command::new("ffmpeg")
        .arg("-help")
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .spawn();

    child.is_ok()
}

/// Directory where the data will be stored.
fn get_data_dir() -> PathBuf {
       let mut data_dir = dirs::data_dir().expect("Failed to create data directory.");
       data_dir.push("listui");
       create_dir_all(data_dir.clone()).expect("Failed to create data directory.");
       data_dir
}

pub fn get_log_path() -> Option<PathBuf> {

    match env::var("LOG_PATH") {
        Ok(var) => PathBuf::from(var).canonicalize().ok(),
        Err(_) => {
            let mut data_dir = get_data_dir();
            data_dir.push("log.txt");
            Some(data_dir)
        }
    }
}

pub fn get_db_path() -> Option<PathBuf> {

    match env::var("DATABASE_PATH") {
        Ok(var) => PathBuf::from(var).canonicalize().ok(),
        Err(_) => {
            let mut data_dir = get_data_dir();
            data_dir.push("db.sqlite");
            Some(data_dir)
        }
    }
}

pub fn get_download_dir() -> Option<PathBuf> {

    match env::var("DOWNLOAD_DIR") {
        Ok(var) => PathBuf::from(var).canonicalize().ok(),
        Err(_) => {
            let mut audio_dir = dirs::audio_dir().expect("Failed to get audio directory.");
            audio_dir.push("listui");
            create_dir_all(audio_dir.clone()).expect("Failed to create data directory.");
            Some(audio_dir)
        }
    }
}

pub fn get_config_path() -> Option<PathBuf> {
    let mut config_dir = dirs::config_dir()?;
    config_dir.push("listui/listui.config");
    Some(config_dir)
}


