use diesel::prelude::*;
use crate::schema::{track, playlist};

pub trait Drawable {
    fn get_text(&self) -> &str;
}

#[derive(Queryable, Identifiable, Debug, Clone)]
#[diesel(table_name = track)]
pub struct Track {
    pub id: i32,
    pub title: String,
    pub yt_id: Option<String>,
    pub playlist_id: Option<i32>
}

impl Drawable for Track {
    
    fn get_text(&self) -> &str {
        &self.title
    }
}

#[derive(Queryable, Identifiable, Debug, Clone)]
#[diesel(table_name = playlist)]
pub struct Playlist {
    pub id: i32,
    pub title: String,
    pub yt_id: String
}

impl Drawable for Playlist {
    
    fn get_text(&self) -> &str {
        &self.title
    }
}

#[derive(Insertable)]
#[diesel(table_name = track)]
pub struct NewVideo {
    pub title: String,
    pub yt_id: String,
    pub playlist_id: Option<i32>
}

#[derive(Insertable)]
#[diesel(table_name = playlist)]
pub struct NewPlaylist {
    pub title: String,
    pub yt_id: String
}