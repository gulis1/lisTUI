
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{ Mutex, Semaphore, SemaphorePermit};


pub enum DownloadResult {
    Completed(PathBuf),
    Failed,
}

pub struct Downloader {

    sem: Arc<Semaphore>,

    /*Hashet containing the youtube IDs of downloads that are either:

      - Still in progress
      - Already completed, but haven't been confirmed by the main thread
        by calling check_for_completed_download.
    */
    downloads: Mutex<HashSet<String>>,

    // The id of the last video the user asked to download. This video will
    // have the top priority in the queue.
    last_download: Mutex<Option<String>> 
}

impl Downloader {

    pub fn new(max_downloads: usize) -> Self {       
        
        Self {

            sem: Arc::new(Semaphore::new(max_downloads)),
            last_download: Mutex::new(None),
            downloads: Mutex::new(HashSet::new()),
        }
    }

    pub async fn download_id(&self, yt_id: &str, file_path: &Path) -> Option<DownloadResult> {

        let mut downloads = self.downloads.lock().await; 
        let mut last_download = self.last_download.lock().await;
        
        last_download.replace(String::from(yt_id));
        if downloads.contains(yt_id) {
            // The video is already in the queue.
            return None;
        }

        downloads.insert(String::from(yt_id));
        drop(downloads);
        drop(last_download);

        let mut permit: SemaphorePermit;
        loop {
            
            permit = self.sem.acquire().await.unwrap();
            let mut last_download = self.last_download.lock().await;
            if last_download.is_none() {
                break;
            }
            else if last_download.as_ref().unwrap() == yt_id {
                last_download.take();
                break;
            }
            
            // Keep waiting in the queue if there is a higher priority download.   
            drop(permit);  
            drop(last_download);          
        }
        
        let child = tokio::process::Command::new("yt-dlp")
            .arg("-x")
            .arg("--audio-format")
            .arg("mp3")
            .arg("-f")
            .arg("bestaudio")
            .arg("--output")
            .arg(file_path)
            .arg("--embed-thumbnail")
            .arg(format!("https://www.youtube.com/watch?v={yt_id}"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        
        drop(permit);

        Some(match child {
            Err(_) =>  DownloadResult::Failed,
            Ok(mut child) => {
                
                let exit = child.wait().await;
                if exit.is_ok() && exit.unwrap().success() { DownloadResult::Completed(file_path.to_path_buf()) }
                else { DownloadResult::Failed }
            }
        })
    }
}

