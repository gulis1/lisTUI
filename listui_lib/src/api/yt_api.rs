/// Module with structs for Youtube's API.

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