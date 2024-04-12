use std::sync::Arc;
use std::path::{PathBuf, Path};
use std::ffi::OsStr;
use std::time::Duration;

use listui_lib::downloader::DownloadResult;
use listui_lib::player::WavWrapper;
use listui_lib::{models::Track, player::Player, downloader::Downloader};
use soloud::{Wav, FromExt};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::sync::MutexGuard;
use tokio::{runtime, task::JoinHandle, sync::{Mutex, mpsc}, time::sleep};
use ratatui::{Frame, layout::{Rect, Layout, Direction, Constraint}, widgets::{Gauge, Borders, Paragraph}, style::Style};

use crate::utils;


#[derive(Debug, Default)]
struct PlayerData {

    current_track: Option<Track>,
    end_timer: Option<JoinHandle<()>>,
    loading_task: Option<JoinHandle<()>>,
    player: Player,
    downloading: bool
}

pub struct PlayerWidget {

    downloader: Arc<Downloader>,
    data: Arc<Mutex<PlayerData>>,
    dir: PathBuf,
    sender: mpsc::Sender<utils::Message>,
    runtime: Arc<runtime::Runtime>
}

impl PlayerWidget {
 
    pub fn new(path: &Path, runtime: Arc<runtime::Runtime>, sender: mpsc::Sender<utils::Message>, max_downloads: usize) -> Self {
        
        Self {
            downloader: Arc::new(Downloader::new(max_downloads)),
            data: Arc::new(Mutex::new(PlayerData::default())),
            dir: path.to_path_buf(),
            sender,
            runtime
        }
    }   

    pub fn play(&mut self, track: Track) {
        
        let mut player_data = self.data.blocking_lock();
        if player_data.current_track.is_some() && player_data.current_track.as_ref().unwrap().id == track.id {
            if player_data.player.get_progress().is_some() {
                drop(player_data);
                self.seek_percentage(0);
            }
            return;
        }
 
        player_data.player.stop();
        player_data.current_track.replace(track.clone());
        if let Some(task) = player_data.loading_task.take() { task.abort();}

        let player_data = Arc::clone(&self.data);
        let mut path = self.dir.clone();
        let downloader = Arc::clone(&self.downloader);
        let runtime = Arc::clone(&self.runtime);
        let sender = self.sender.clone();
        self.runtime.spawn(async move {
            
            let mut filename = if track.yt_id.is_some() { track.title.replace(['/', '\\', ':', '*', '<', '>', '|', '\"'], "") }
                else { track.title.clone()};

            filename.push_str(".mp3");
            path.push(OsStr::new(&filename));
            if !path.exists() { 
                let yt_id = track.yt_id.expect("No youtube id available.");
                let mut guard = player_data.lock().await;
                guard.downloading = true;
                drop(guard);
                let res = downloader.download_id(&yt_id, &path).await;
                match res {
                    None => return, // Another task is trying to play this track.
                    Some(DownloadResult::Failed) => {
                        sender.send(utils::Message::SongFinished).await.expect("Failed to send message.");
                        return;
                    },
                    _ => {}
                }
            }
            
            let mut data_guard = player_data.lock().await;
            if data_guard.current_track.is_some() && data_guard.current_track.as_ref().unwrap().id == track.id { 

                data_guard.downloading = false;
                data_guard.player.stop();
                if let Some(timer) = data_guard.end_timer.take() { timer.abort(); }

                let rt = Arc::clone(&runtime);
                let data = Arc::clone(&player_data);
                data_guard.loading_task.replace(runtime.spawn(async move {
                    
                    let mut file = File::open(path).await.unwrap();
                    let file_size = file.metadata().await.unwrap().len() as usize;
                    let mut buffer: Vec<u8> = vec![0; file_size];
                    file.read_exact(&mut buffer).await.unwrap();
                    
                    // Processing the audio file can take a few seconds if it is large.
                    // This can be a problem because the task cannot be aborted while it is
                    // loading.
                    let wav = WavWrapper(Wav::from_mem(&buffer).unwrap());

                    // Very ugly workaround for the previous comment.
                    sleep(Duration::from_millis(10)).await;

                    let mut data_guard = data.lock().await;  
                    let loading_result = data_guard.player.play_wav(wav);
                    if loading_result.is_err() {
                        sender.send(utils::Message::SongFinished).await.expect("Failed to send message.");
                        return;
                    }

                    set_timer(&rt, &mut data_guard, sender);
                }));
            }
        });
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
       
        let data_guard = self.data.blocking_lock();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Length(area.height - 2)].as_ref())
            .split(area);

        let title = match data_guard.current_track.as_ref() {
            Some(s) => s.title.as_str(),
            None => "No song selected."
        };

        let (label, ratio) = {
            
            match data_guard.player.get_progress() {
                Some(progress) => {
                    let duration = data_guard.player.get_duration() as i32;
                    let label = utils::time_str(progress as i32, duration, data_guard.player.is_paused());
                    let ratio = progress as f64 / duration as f64;
                    (label, ratio)
                },
                None => { 
                    if data_guard.downloading { (String::from("Downloading..."), 0.0) }
                    else { (String::new(), 0.0) }
                }
            }
        };
        
        let gauge = Gauge::default()
            .block(super::BLOCK.clone().borders(Borders::ALL ^ Borders::BOTTOM).title(title))
            .gauge_style(Style::default().fg(super::ACC_COLOR))
            .ratio(ratio)
            .label(label);
                
        let p = Paragraph::new(format!("\nVolume: {}% (press H for help)", data_guard.player.get_volume()))
            .block(super::BLOCK.clone().borders(Borders::ALL ^ Borders::TOP));
    
        frame.render_widget(gauge, chunks[0]);
        frame.render_widget(p, chunks[1]);

    }

    pub fn stop(&mut self) {
        let mut data = self.data.blocking_lock();
        data.downloading = false;
        stop_timer(&mut data);
        data.current_track.take();
        data.player.stop();
    }

    pub fn toggle_pause(&mut self) {

        let mut data = self.data.blocking_lock();
        if data.player.is_paused() { 
            data.player.resume(); 
            set_timer(&self.runtime, &mut data, self.sender.clone()); 
        }
        else {
            stop_timer(&mut data);
            data.player.pause();
        }
    }

    pub fn increase_volume(&mut self, inc: i32) {
        self.data.blocking_lock().player.increase_volume(inc);
    }

    pub fn decrease_volume(&mut self, inc: i32) {
        self.data.blocking_lock().player.decrease_volume(inc);
    }

    pub fn seek_percentage(&mut self, pcent: u64) {
        
        let mut guard = self.data.blocking_lock();
        if guard.player.is_playing() {
            guard.player.seek_percentage(pcent);
            set_timer(&self.runtime, &mut guard, self.sender.clone());
        }
    }

    pub fn forward(&mut self, seconds: u64)  {

        let mut guard = self.data.blocking_lock();
        if guard.player.is_playing() {
            guard.player.forward(seconds);
            set_timer(&self.runtime, &mut guard, self.sender.clone());
        }       
    }

    pub fn rewind(&mut self, seconds: u64) {
        
        let mut guard = self.data.blocking_lock();
        if guard.player.is_playing() {
            guard.player.rewind(seconds);
            set_timer(&self.runtime, &mut guard, self.sender.clone());
        }
    }
}

fn set_timer(runtime: &runtime::Runtime, data: &mut MutexGuard<PlayerData>, sender: mpsc::Sender<utils::Message>) {
    
    stop_timer(data);

    let duration = data.player.get_duration();
    let seconds = duration - data.player.get_progress().unwrap_or(duration) + 1;

    data.end_timer.replace(runtime.spawn(async move {
        sleep(Duration::from_secs(seconds + 1)).await;
        sender.send(utils::Message::SongFinished).await.expect("TODO: remove expect");
    }));   
}

#[inline]
fn stop_timer(data: &mut MutexGuard<PlayerData>) {

    if let Some(timer) = data.end_timer.take() { timer.abort() }
}
