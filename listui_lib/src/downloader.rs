
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::runtime;
use tokio::sync::{mpsc, Mutex, Semaphore, SemaphorePermit};


pub enum DownloadResult {
    Completed(String, PathBuf),
    Failed(String)
}

/* My (probably "overengineered") implementation of a youtube downloader.
// Currently used yt_dlp to download the video, and ffmpeg to extract the audio. (Maybe
   it would be cool to use Symphonia in the future).

   It allows setting a maximum number of concurrent downloads, and it prioritizes the last
   video that was queued. It also won't queue a video again if it's queued (it will still
   get top priotiy though)
*/
pub struct Downloader {

    runtime: runtime::Runtime,
    sem: Arc<Semaphore>,

    /*Hashet containing the youtube IDs of downloads that are either:

      - Still in progress
      - Already completed, but haven't been confirmed by the main thread
        by calling check_for_completed_download.
    */
    downloads: RefCell<HashSet<String>>,

    // The id of the last video the user asked to download. This video will
    // have the top priority in the queue.
    last_download: Arc<Mutex<Option<String>>>,  

    receiver: RefCell<mpsc::Receiver<DownloadResult>>,
    sender: mpsc::Sender<DownloadResult>
}

impl Downloader {

    pub fn new(max_downloads: usize) -> Self {

        let runtime = runtime::Builder::new_multi_thread()     
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("Failed to obtain tokio runtime.");
        
        let (sender, receiver) = mpsc::channel::<DownloadResult>(max_downloads);
        
        Self {

            runtime,
            sem: Arc::new(Semaphore::new(max_downloads)),
            last_download: Arc::new(Mutex::new(None)),
            downloads: RefCell::new(HashSet::new()),
            receiver: RefCell::new(receiver),
            sender
        }
    }

    // Returns the status of the last finished download.
    pub fn check_for_completed_download(&self) -> Option<DownloadResult> {

        match self.receiver.borrow_mut().try_recv() {
            Err(_) => None, // No downloads finished since the last time the functions was called.
            Ok(d) => {
                let mut last_download = self.last_download.blocking_lock();
                let downloaded_id = match &d {
                    DownloadResult::Completed(s, _) | DownloadResult::Failed(s) => s,
                };

                self.downloads.borrow_mut().remove(downloaded_id);

                if last_download.is_some() && last_download.as_ref().unwrap() == downloaded_id {
                    last_download.take();
                }

                Some(d)
            },
        }
    }

    pub fn download_url(&self, yt_id: &str, file_path: &Path) {

        let mut downloads = self.downloads.borrow_mut(); 
        let mut guard = self.last_download.blocking_lock();
        
        guard.replace(String::from(yt_id));
        if downloads.contains(yt_id) {
            // The video is already in the queue.
            return;
        }

        downloads.insert(String::from(yt_id));

        // Cloning stuff to send it to another thread.
        let sender = self.sender.clone();
        let id = String::from(yt_id);
        let sem = Arc::clone(&self.sem);
        let mtx = Arc::clone(&self.last_download);
        let file_path = file_path.to_path_buf();

        self.runtime.spawn(async move {

            let mut permit: SemaphorePermit;
            loop {
                
                permit = sem.acquire().await.unwrap();
                let mut last_download = mtx.lock().await;
                if last_download.is_none() {
                    break;
                }
                else if last_download.as_ref().unwrap() == &id {
                    last_download.take();
                    break;
                }
                
                // Keep waiting in the queue if there is a higher priority download.   
                drop(permit);  
                drop(last_download);          
            }
            
            let mut child = tokio::process::Command::new("yt-dlp")
                .arg("-x")
                .arg("--audio-format")
                .arg("mp3")
                .arg("-f")
                .arg("bestaudio")
                .arg("--output")
                .arg(&file_path)
                .arg("--embed-thumbnail")
                .arg(format!("https://www.youtube.com/watch?v={id}"))
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap();
            
            let exit = child.wait().await;
            drop(permit);
            
            if exit.is_ok() && exit.unwrap().success() { let _ = sender.send(DownloadResult::Completed(id, file_path)).await; }
            else { let _ = sender.send(DownloadResult::Failed(id)).await;}
        });
    }
}

