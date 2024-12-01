use anyhow::Result;
use std::sync::Arc;
use std::path::{PathBuf, Path};
use std::ffi::OsStr;
use std::time::Duration;

use listui_lib::downloader::DownloadResult;
use listui_lib::{models::Track, player::Player, downloader::Downloader};
use tokio::sync::MutexGuard;
use tokio::{runtime, task::JoinHandle, sync::{Mutex, mpsc}, time::sleep};
use ratatui::{Frame, layout::{Rect, Layout, Direction, Constraint}, widgets::{Gauge, Borders, Paragraph}, style::Style};

use crate::utils;


#[derive(Debug, Default)]
struct PlayerData {

    current_track: Option<Track>,
    end_timer: Option<JoinHandle<()>>,
    downloading: bool
}

pub struct PlayerWidget {

    downloader: Arc<Downloader>,
    data: Arc<Mutex<PlayerData>>,
    dir: PathBuf,
    sender: mpsc::Sender<utils::Message>,
    runtime: Arc<runtime::Runtime>,
    player: Arc<Player>
}

impl PlayerWidget {
 
    pub fn try_new(path: &Path, runtime: Arc<runtime::Runtime>, sender: mpsc::Sender<utils::Message>, max_downloads: usize) -> Result<Self> {
        
        Ok(Self {
            downloader: Arc::new(Downloader::new(max_downloads)),
            data: Arc::new(Mutex::new(PlayerData::default())),
            dir: path.to_path_buf(),
            sender,
            runtime,
            player: Arc::new(Player::try_default()?)
        })
    }   

    pub fn play(&mut self, track: Track) {
        
        let mut player_data = self.data.blocking_lock();
        if player_data.current_track.is_some() && player_data.current_track.as_ref().unwrap().id == track.id {
            if self.player.is_playing() {
                drop(player_data);
                self.seek_percentage(0);
            }
            return;
        }
 
        self.player.stop();
        player_data.current_track.replace(track.clone());
        
        let player = Arc::clone(&self.player);
        let player_data = Arc::clone(&self.data);
        let mut path = self.dir.clone();
        let downloader = Arc::clone(&self.downloader);
        let sender = self.sender.clone();
        let runtime = Arc::clone(&self.runtime);
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
                player.stop();
                if let Some(timer) = data_guard.end_timer.take() { timer.abort(); }
                
                if let Err(_e) = player.play_file(&path) {
                    // TODO: log error
                    sender.send(utils::Message::SongFinished).await.expect("Failed to send message.");
                    return;
                }
                else {
                    set_timer(&player, &runtime, &mut data_guard, sender);
                }
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
            
            match self.player.get_progress() {
                None => {
                    if data_guard.downloading { (String::from("Downloading..."), 0.0) }
                    else { (String::new(), 0.0) }
                },
                Some(progress) => {
                    let duration = self.player.get_duration() as i32;
                    if duration != 0 {
                        let label = utils::time_str(progress as i32, duration, self.player.is_paused());
                        let ratio = progress as f64 / duration as f64;
                        (label, ratio)
                    }
                    else { (String::new(), 0.0) }
                }
            }
        };
        
        let gauge = Gauge::default()
            .block(super::BLOCK.clone().borders(Borders::ALL ^ Borders::BOTTOM).title(title))
            .gauge_style(Style::default().fg(super::ACC_COLOR))
            .ratio(ratio)
            .label(label);
                
        let p = Paragraph::new(format!("\nVolume: {}% (press H for help)", self.player.get_volume()))
            .block(super::BLOCK.clone().borders(Borders::ALL ^ Borders::TOP));
    
        frame.render_widget(gauge, chunks[0]);
        frame.render_widget(p, chunks[1]);

    }

    pub fn stop(&mut self) {
        let mut data = self.data.blocking_lock();
        data.downloading = false;
        stop_timer(&mut data);
        data.current_track.take();
        self.player.stop();
    }

    pub fn toggle_pause(&mut self) {

        let mut data = self.data.blocking_lock();
        if self.player.is_paused() { 
            self.player.resume(); 
            set_timer(&self.player, &self.runtime, &mut data, self.sender.clone()); 
        }
        else {
            stop_timer(&mut data);
            self.player.pause();
        }
    }

    pub fn increase_volume(&mut self, inc: i32) {
        self.player.increase_volume(inc);
    }

    pub fn decrease_volume(&mut self, inc: i32) {
        self.player.decrease_volume(inc);
    }

    pub fn seek_percentage(&mut self, pcent: u64) {
        
        let mut guard = self.data.blocking_lock();
        if self.player.is_playing() {
            self.player.seek_percentage(pcent);
            set_timer(&self.player, &self.runtime, &mut guard, self.sender.clone());
        }
    }

    pub fn forward(&mut self, seconds: u64)  {

        let mut guard = self.data.blocking_lock();
        if self.player.is_playing() {
            self.player.forward(seconds);
            set_timer(&self.player, &self.runtime, &mut guard, self.sender.clone());
        }       
    }

    pub fn rewind(&mut self, seconds: u64) {
        
        let mut guard = self.data.blocking_lock();
        if self.player.is_playing() {
            self.player.rewind(seconds);
            set_timer(&self.player, &self.runtime, &mut guard, self.sender.clone());
        }
    }
}

fn set_timer(player: &Arc<Player>, runtime: &runtime::Runtime, data: &mut MutexGuard<PlayerData>, sender: mpsc::Sender<utils::Message>) {
    
    stop_timer(data);
    
    let duration = player.get_duration();
    let seconds = duration - player.get_progress().unwrap_or(duration) + 1;
    //println!("{}, {:?}, {}", duration, player.get_progress(), seconds);
    data.end_timer.replace(runtime.spawn(async move {
        sleep(Duration::from_secs(seconds + 1)).await;
        sender.send(utils::Message::SongFinished).await.expect("TODO: remove expect");
    }));   
}

#[inline]
fn stop_timer(data: &mut MutexGuard<PlayerData>) {

    if let Some(timer) = data.end_timer.take() { timer.abort() }
}
