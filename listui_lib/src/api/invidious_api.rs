/// Module with structs for Invidious' API.


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