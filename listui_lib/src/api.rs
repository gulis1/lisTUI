use reqwest;
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

#[derive(Debug)]
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

// Modules that contain serde structs for each api response.
mod yt_api {

    use serde::{Serialize, Deserialize};

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct PageInfo {
        pub total_results: i32,
        pub results_per_page: i32
    }
    
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct ResourceId {
        pub kind: String,
        pub video_id: String
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Snippet {
        pub title: String,
        pub resource_id: Option<ResourceId>
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Item {
        pub snippet: Snippet,
        pub id: String
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct ApiResponse {
        pub page_info: PageInfo,
        pub items: Vec<Item>,
        pub next_page_token: Option<String>
    }
}

mod invidious_api {

    use serde::{Serialize, Deserialize};

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Video {

        pub title: String,
        pub video_id: String,
        pub index: i32
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct PlaylistResponse {
        pub title: String,
        pub playlist_id: String,
        pub videos: Vec<Video>,
    }
}    

// All requests to Youtube's or Invidious' api are made here.
pub struct ApiClient {

    client: reqwest::blocking::Client,
    api_key: Option<String>,
}

impl ApiClient {

    pub fn from_youtube(api_key: String) -> Self {

        Self {
            client: reqwest::blocking::Client::new(),
            api_key: Some(api_key)
        }
    }

    pub fn from_invidious() -> Self {

        Self {
            client: reqwest::blocking::Client::new(),
            api_key: None
        }
    }
   
    pub fn fetch_playlist(&self, yt_id: &str) -> Result<(NewPlaylist, Vec<NewVideo>), ApiError> {

        match self.api_key {
            Some(_) => {
                let playlist = self.fetch_youtube_playlist_info(yt_id)?;
                let videos = self.fetch_youtube_videos(&playlist.yt_id)?;
                Ok((playlist, videos))
            },
            None => {
                // Loop through invidious instances, in case some of them are down.
                let mut r: Result<(NewPlaylist, Vec<NewVideo>), ApiError> = Err(ApiError::Unknown);
                for i in INVIDIOUS_INSTANCES {
                    r = self.fetch_invidious_playlist(i, yt_id);
                    if r.is_ok() { break }
                }

                r       
            }
        }
    }

    fn fetch_youtube_playlist_info(&self,  yt_id: &str) -> Result<NewPlaylist, ApiError> {

        let mut content = self.client.get(format!("{}/playlists?part=snippet&key={}&id={}", YOUTUBE_API_URL, self.api_key.as_ref().unwrap(), yt_id)).send()
        .map_err(convert_reqwest_err)
        .and_then(|response| {
            serde_json::from_str::<yt_api::ApiResponse>(&response.text_with_charset("utf-8")
                .map_err(|_| ApiError::DecodingError)?)
                .map_err(|_| ApiError::ParsingError)
        })?;

        if content.items.len() == 1 {
            
            let playlist = content.items.remove(0);
            Ok(NewPlaylist {
                title: playlist.snippet.title,
                yt_id: playlist.id
            })  
        }
        else { Err(ApiError::NotFoundError(String::from(yt_id))) }
    }

    fn fetch_youtube_videos(&self, playlist_ytid: &str) -> Result<Vec<NewVideo>, ApiError> {
        // Fetches all videos from the given youtube playlist.

        let mut videos: Vec<NewVideo> = Vec::new();
        let mut next_page_token: Option<String> = None;
        loop {
            
            let mut url = format!("{}/playlistItems?maxResults=50&part=snippet&key={}&playlistId={}", YOUTUBE_API_URL, self.api_key.as_ref().unwrap(), playlist_ytid);
            if let Some(token) = next_page_token {
                url.push_str(&format!("&pageToken={token}"));
            }

            let content = self.client.get(url).send()
            .map_err(convert_reqwest_err)
            .and_then(|response| {
                serde_json::from_str::<yt_api::ApiResponse>(&response.text_with_charset("utf-8")
                    .map_err(|_| ApiError::DecodingError)?)
                    .map_err(|_| ApiError::ParsingError)
            })?;

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
 
            next_page_token = content.next_page_token;
            if next_page_token.is_none() { break; }
        }

        Ok(videos)
    }

    fn fetch_invidious_playlist(&self, instance: &str, yt_id: &str) -> Result<(NewPlaylist, Vec<NewVideo>), ApiError> {
        
        let mut videos: Vec<NewVideo> = Vec::new();
        let mut page: i32 = 1;
        let mut last_index: i32 = -1;
        let mut playlist: NewPlaylist;

        loop {
            
            let content = self.client.get(format!("{}/api/v1/playlists/{}?page={}", instance, yt_id, page)).send()
            .map_err(convert_reqwest_err)
            .and_then(|response| {
                serde_json::from_str::<invidious_api::PlaylistResponse>(&response.text_with_charset("utf-8")
                    .map_err(|_| ApiError::DecodingError)?)
                    .map_err(|_| ApiError::ParsingError)
            })?;

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
            
            last_index = x;
            page += 1;
        }

        Ok((playlist, videos))
    }
}

fn convert_reqwest_err(err: reqwest::Error) -> ApiError {

    match err.status() {
        Some(err) => { ApiError::RequestError(err.to_string())},
        None => ApiError::Unknown,
    }
}
