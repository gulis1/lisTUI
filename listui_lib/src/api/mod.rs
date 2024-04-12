/// Module with structs for Invidious's API.

mod yt_api;
mod invidious_api;

use reqwest::{self, Response};
use crate::models::{NewPlaylist, NewVideo};

const YOUTUBE_API_URL: &str = "https://www.googleapis.com/youtube/v3";

// TODO: make this configurable.
static INVIDIOUS_INSTANCES: [&str; 5] =  [
    "https://vid.puffyan.us",
    "https://y.com.sb",
    "https://invidious.nerdvpn.de",
    "https://invidious.tiekoetter.com",
    "https://inv.bp.projectsegfau.lt"
];

#[derive(Debug, Clone)]
pub enum ApiError {
    
    NotFoundError(String),
    RequestError(String),
    DecodingError,
    ParsingError,
    Unknown
}

impl std::error::Error for ApiError {}
impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::NotFoundError(id) => write!(f, "Couldn't find playlist with id {id}."),
            ApiError::RequestError(err) => write!(f, "{}", err),
            ApiError::DecodingError | ApiError::ParsingError => write!(f, "Failed to parse api response."),
            ApiError::Unknown => write!(f, "Unknown error.")
        }
    }
}

pub type ApiProgressCallback = Box<dyn Fn(String) + Send + Sync>;

/// A `reqwest::Client` wrapper, that can query videos either from ỲouTube
/// or Invidious.
/// 
/// Because playlist queries can take a quite a while, the user can define a callback
/// function that will be called multiple times with a `String` with information
/// about the progress.
pub struct ApiClient {
    client: reqwest::Client,
    api_key: Option<String>,
    callback: Option<ApiProgressCallback>
}

impl ApiClient {

    /// Crates a new YouTube client.
    /// 
    /// If a callback is provided, it will be called multiple times with information
    /// about the progress.
    pub fn from_youtube(api_key: String, callback: Option<ApiProgressCallback>) -> Self {

        Self {
            client: reqwest::Client::new(),
            api_key: Some(api_key),
            callback
        }
    }

    /// Crates a new Invidious client.
    /// 
    /// If a callback is provided, it will be called multiple times with information
    /// about the progress.
    pub fn from_invidious(callback: Option<ApiProgressCallback>) -> Self {

        Self {
            client: reqwest::Client::new(),
            api_key: None,
            callback
        }
    }
    
    /// Tries to fetch the information about all videos from a YouTube playlist.
    /// 
    /// Depending if `self` was created using `Self::from_youtube` or `Self::from_invidious`, 
    /// the information will be fetched from either YouTube or Invidious.
    pub async fn fetch_playlist(&self, yt_id: &str) -> Result<(NewPlaylist, Vec<NewVideo>), ApiError> {

        if self.api_key.is_some() {
            self.send_callback(format!("Fetching playlist {yt_id} from YouTube."));
            let playlist = self.fetch_youtube_playlist_info(yt_id).await?;
            let videos = self.fetch_youtube_videos(&playlist.yt_id).await?;
            Ok((playlist, videos))
        }
        else {
            // Loop through invidious instances, in case some of them are down.
            let mut r: Result<(NewPlaylist, Vec<NewVideo>), ApiError> = Err(ApiError::Unknown);
            for instance in INVIDIOUS_INSTANCES {
                self.send_callback(format!("Fetching playlist {yt_id} from Invidious instance: {instance}"));
                r = self.fetch_invidious_playlist(instance, yt_id).await;
                match &r {
                    Ok(_) => break,
                    Err(e) => self.send_callback(format!("Cloud not fetch playlist {yt_id} from {instance}: {e}"))    
                }
            }
            r   
        }
    }

    /// Gets a playlist's title using Youtube's API.
    async fn fetch_youtube_playlist_info(&self,  yt_id: &str) -> Result<NewPlaylist, ApiError> {

        let response = self.client.get(format!("{}/playlists?part=snippet&key={}&id={}", YOUTUBE_API_URL, self.api_key.as_ref().unwrap(), yt_id))
            .send().await
            .map_err(convert_reqwest_err)?;
    
        let mut content = parse_youtube_response(response).await?;
        if content.items.len() == 1 {
            
            let playlist = content.items.remove(0);
            Ok(NewPlaylist {
                title: playlist.snippet.title,
                yt_id: playlist.id
            })  
        }
        else { Err(ApiError::NotFoundError(String::from(yt_id))) }
    }

    /// Gets information about all songs in a playlist, using Youtube's API.
    async fn fetch_youtube_videos(&self, playlist_ytid: &str) -> Result<Vec<NewVideo>, ApiError> {

        let mut videos: Vec<NewVideo> = Vec::new();
        let mut next_page_token: Option<String> = None;
        loop {
            
            let mut url = format!("{}/playlistItems?maxResults=50&part=snippet&key={}&playlistId={}", 
                YOUTUBE_API_URL,
                self.api_key.as_ref().unwrap(), 
                playlist_ytid
            );

            if let Some(token) = next_page_token {
                url.push_str(&format!("&pageToken={token}"));
            }

            let response = self.client.get(url)
                .send().await
                .map_err(convert_reqwest_err)?;

            let content = parse_youtube_response(response).await?;
            videos.extend(content.items.into_iter()
                .filter(|v| v.snippet.title != "Deleted video" && v.snippet.title  != "Private video" && v.snippet.resource_id.is_some())
                .filter_map(|v|{
                    Some(NewVideo {
                        title: v.snippet.title,
                        yt_id: v.snippet.resource_id.ok_or(ApiError::ParsingError).ok()?.video_id,
                        playlist_id: None
                    })
                })
            );

            self.send_callback(format!("Fetched {} videos.", videos.len()));
 
            next_page_token = content.next_page_token;
            if next_page_token.is_none() { break; }
        }

        Ok(videos)
    }

    /// Gets both a playlist's title and all its videos using Youtube's API.
    async fn fetch_invidious_playlist(&self, instance: &str, yt_id: &str) -> Result<(NewPlaylist, Vec<NewVideo>), ApiError> {
        
        let mut videos: Vec<NewVideo> = Vec::new();
        let mut page: i32 = 1;
        let mut last_index: i32 = -1;
        let mut playlist: NewPlaylist;

        loop {
            
            let response = self.client.get(format!("{}/api/v1/playlists/{}?page={}", instance, yt_id, page)).send().await
                .map_err(convert_reqwest_err)?;

            let content = parse_invidious_reponse(response).await?;
            playlist = NewPlaylist {
                title: content.title,
                yt_id: content.playlist_id
            };

            if content.videos.is_empty() { break; }

            // Invidious api paging is a bit weird, and it can return the same videos in multiple pages.
            // To prevent saving the same video multiple times, the index of the last song in the previous
            // query is saved, and then it's used to filter the videos in the next query.
            let x = content.videos.last().unwrap().index;  

            videos.extend(content.videos.into_iter()
            .filter(|v| v.index > last_index && v.title != "[Deleted video]" && v.title  != "[Private video]")
            .map(|v| {
                NewVideo {
                    title: v.title,
                    yt_id: v.video_id,
                    playlist_id: None
                }
            }));

            self.send_callback(format!("Fetched {} videos.", videos.len()));

            last_index = x;
            page += 1;
        }

        Ok((playlist, videos))
    }

    fn send_callback(&self, progress: String) {
        log::info!("{progress}");
        if let Some(callback) = &self.callback {
            callback(progress);
        }
    }
}

async fn parse_youtube_response(response: Response) -> Result<yt_api::ApiResponse, ApiError> {

    serde_json::from_str::<yt_api::ApiResponse>(&response.text_with_charset("utf-8").await
        .map_err(|_| ApiError::DecodingError)?)
        .map_err(|_| ApiError::ParsingError)
}

async fn parse_invidious_reponse(response: Response) -> Result<invidious_api::PlaylistResponse, ApiError> {

    serde_json::from_str::<invidious_api::PlaylistResponse>(&response.text_with_charset("utf-8").await
        .map_err(|_| ApiError::DecodingError)?)
        .map_err(|_| ApiError::ParsingError)
}

fn convert_reqwest_err(err: reqwest::Error) -> ApiError {

    match err.status() {
        Some(err) => { ApiError::RequestError(err.to_string())},
        None => ApiError::Unknown,
    }
}