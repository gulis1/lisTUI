
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{ Mutex, Semaphore, SemaphorePermit};


pub enum DownloadResult {
    Completed(PathBuf),
    Failed,
}

/// Client to download videos from YouTube, using `yt-dlp`.
/// 
/// The client keeps track of previously enqueued videos, so
/// it doesn't download the same video twice. 
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
    last_enqueued: Mutex<Option<String>> 
}

impl Downloader {

    /// Creates a new client that can download up to `max_downloads` simultaneously.
    pub fn new(max_downloads: usize) -> Self {       
        
        Self {
            sem: Arc::new(Semaphore::new(max_downloads)),
            last_enqueued: Mutex::new(None),
            downloads: Mutex::new(HashSet::new()),
        }
    }

    /// Download a video with a given youtube ID.
    /// 
    /// If there are other enqueued videos, the last newly enqueued one will have priority.
    pub async fn download_id(&self, yt_id: &str, file_path: &Path) -> Option<DownloadResult> {

        let mut downloads = self.downloads.lock().await; 
        let mut last_enqueued = self.last_enqueued.lock().await;
        
        last_enqueued.replace(String::from(yt_id));
        if downloads.contains(yt_id) {
            // Early return if the video is already enqueued.
            log::info!("Video {yt_id} was already enqueued.");
            return None;
        }

        log::info!("Enqueued video {yt_id}.");
        downloads.insert(String::from(yt_id));
        drop(downloads);
        drop(last_enqueued);

        let mut permit: SemaphorePermit;
        loop {
            
            permit = self.sem.acquire().await.unwrap();
            let mut last_download = self.last_enqueued.lock().await;
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
        
        log::info!("Starting download for video {yt_id}");
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
            // The download did not even start.
            Err(e) =>  {
                log::error!("Download for video {yt_id} failed: {e}");
                DownloadResult::Failed
            },
            Ok(mut child) => {
                
                match child.wait().await.map(|exit| exit.success()) {
                    Ok(true) => {
                        log::info!("Download for video {yt_id} completed succesfully.");
                        DownloadResult::Completed(file_path.to_path_buf())
                    },
                    Ok(false) | Err(_) => {
                        DownloadResult::Failed
                    }
                }
            }
        })
    }
}

