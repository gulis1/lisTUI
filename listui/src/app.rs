use listui_lib::db::{Dao, DbError};

use listui_lib::models::{Playlist, Track};
use listui_lib::player::Player;
use listui_lib::downloader::{Downloader, DownloadResult};

use tui::Frame;
use tui::Terminal;
use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::Style;
use tui::widgets::{Borders, Gauge, Paragraph};

use std::error::Error;
use std::ffi::OsStr;
use std::io::Stdout;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossterm::event;
use crossterm::event::{Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use rand::prelude::*;

use crate::widgets::*;
use crate::utils::time_str;

#[derive(Clone, Copy)]
pub enum CurrentScreen {

    Playlists,
    Songs,
    SongControls
}

#[derive(Clone, Copy)]
enum SelectionMode {

    Follow,
    Manual
}

pub struct ListuiApp {

    current_screen: CurrentScreen,
    playlists_widget: ListWidget<Playlist>,
    songs_widget: ListWidget<Track>,
    
    player: Player,
    dao: Option<Dao>,
    downloader: Downloader,

    playlist_dir: PathBuf,

    current_playlist: Option<String>,
    current_song: Option<Track>,
    current_song_ind: Option<usize>,
    songs_selmode: SelectionMode,

    search_query: String,
    shuffle: bool,
    downloading: bool
}

impl ListuiApp {

    pub fn new(playlist_dir: PathBuf, database_path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {

        let dao = Dao::new(&database_path)?;

        Ok(Self {
           
            current_screen: CurrentScreen::Playlists,
            playlists_widget: ListWidget::with_items(dao.get_playlists()?),
            songs_widget: ListWidget::empty(),
            
            player: Player::default(),
            dao: Some(dao),
            downloader: Downloader::new(3), // TODO: make max_downloads configurable.

            playlist_dir,

            current_playlist: None,
            current_song: None,
            current_song_ind: None,
            songs_selmode: SelectionMode::Follow,
            search_query: String::new(),
            shuffle: false,
            downloading: false
        })
    }

    pub fn new_open_playlist(playlist_dir: PathBuf, database_path: PathBuf, playlist_id: i32) -> Result<Self, Box<dyn std::error::Error>> {

        let mut app = ListuiApp::new(playlist_dir, database_path)?;

        app.load_songs(playlist_id)?;
        app.current_screen = CurrentScreen::Songs;

        Ok(app)
    }

    pub fn with_tracks(playlist_dir: PathBuf, tracks: Vec<Track>) -> Result<Self, Box<dyn std::error::Error>> {

        let playlist_name = (|| {
            Some(playlist_dir.file_name()?.to_string_lossy().to_string())
        })().unwrap_or(String::from("Unknown playlist."));

        Ok(Self {
           
            current_screen: CurrentScreen::Songs,
            playlists_widget: ListWidget::empty(),
            songs_widget: ListWidget::with_items(tracks),
            
            player: Player::default(),
            dao: None,
            downloader: Downloader::new(3), // TODO: make max_downloads configurable.

            playlist_dir,

            current_playlist: Some(playlist_name),
            current_song: None,
            current_song_ind: None,
            songs_selmode: SelectionMode::Follow,
            search_query: String::new(),
            shuffle: false,
            downloading: false
        })
    }

    pub fn run(&mut self) -> Result<(),  Box<dyn Error>> {

        let tick_rate = Duration::from_millis(500); // TODO: add config for this.
        let mut last_tick = Instant::now();

        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        loop {

            terminal.draw(|f| self.draw(f))?;

            if !self.player.is_playing() && self.current_song_ind.is_some() && !self.downloading { self.play_next(); }  
            self.check_song_received();

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));
            
            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if self.process_input(key.code) { break; }
                }
            }
            if last_tick.elapsed() >= tick_rate { last_tick = Instant::now(); }
        }
        
        disable_raw_mode()?;
        terminal.backend_mut();

        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
        )?;
         
        terminal.show_cursor()?;   

        Ok(())
    }

    fn check_song_received(&mut self) {

        if self.downloading {
            // Check if the download has finished.
            match self.downloader.check_for_completed_download() {
                Some(DownloadResult::Completed(id, path)) => {
                    // The download that just finished is the one we were waiting for.
                    let yt_id = self.current_song.as_ref().and_then(|song| song.yt_id.as_ref());
                    if let Some(yt_id) = yt_id {
                        if &id == yt_id{ 
                            let _ = self.player.play_file(&path); 
                            self.downloading = false;
                        }
                    }
                },
                Some(DownloadResult::Failed(_)) => self.play_next(),
                None => {}, // The download that just finished is an older one.
            }
        }   
    }
    
    fn load_songs(&mut self, playlist_id: i32) -> Result<(), DbError> {
        
        // Loads from the DB all tracks of the playlist with the given id.
        if let Some(ref dao) = self.dao {
            let playlist = dao.get_playlist(playlist_id)?;
            let songs = dao.get_tracks(playlist_id)?;
            self.current_playlist = Some(playlist.title);
            self.songs_widget = ListWidget::with_items(songs);
    
            Ok(())
        }
        else { Err(DbError::ConnectionError) }
    }

    fn draw(&mut self, frame: &mut Frame<CrosstermBackend<Stdout>>) {
        
        if frame.size().width < 25{ draw_not_enough_width(frame); }
        else if frame.size().height < 10 { draw_not_enough_height(frame) }
        else { match self.current_screen  {

            CurrentScreen::Playlists => self.draw_playlists(frame, frame.size()),
            CurrentScreen::Songs => self.draw_songs(frame, frame.size()),
            CurrentScreen::SongControls => draw_controls_screen(frame, frame.size())
        }}  
    }    

    fn draw_playlists(&mut self, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {

        if area.height < 20 || area.width < 50 {
            self.playlists_widget.draw(frame, area, "Playlists");
        }
        else {

            let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(10), Constraint::Length(area.height - 10)].as_ref())
            .split(area);

            draw_logo(frame, chunks[0]);
            self.playlists_widget.draw(frame, chunks[1], "Playlists");
        }
    }

    fn draw_songs(&mut self, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {

        let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(area.height - 5), Constraint::Length(5)].as_ref())
                .split(area);

        if self.songs_widget.is_filtered() {
            let s = format!(" 🔎︎ Search: {} ", self.search_query);
            self.songs_widget.draw(frame, chunks[0], &s);
        }
        else {
            let plist_title = self.current_playlist.as_ref().unwrap().as_str();
            self.songs_widget.draw(frame, chunks[0], plist_title);
        }
        
        self.draw_progress_bar(frame, chunks[1]);
    }

    fn draw_progress_bar(&mut self, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
        
        let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(2), Constraint::Length(area.height - 2)].as_ref())
                .split(area);

        // TODO: maybe move this to a new struct inside the widget module?
        let mut title = "No song selected.";
        if let Some(song) = self.current_song.as_ref() { title = &song.title; }
   
        let (label, ratio) = {
            
            match self.player.get_progress() {
                Some(progress) => {
                    let duration =  self.player.get_duration() as i32;
                    (time_str(progress as i32, duration), progress as f64 / duration as f64)
                }
                None => { 
                    if self.downloading { (String::from("Downloading..."), 0.0) }
                    else { (String::new(), 0.0) }
                }
            }
        };
        
        let gauge = Gauge::default()
            .block(BLOCK.clone().borders(Borders::ALL ^ Borders::BOTTOM).title(title))
            .gauge_style(Style::default().fg(ACC_COLOR))
            .ratio(ratio)
            .label(label);
                
        let p = Paragraph::new(format!("\nVolume: {}% (press H for help)", self.player.get_volume()))
            .block(BLOCK.clone().borders(Borders::ALL ^ Borders::TOP));
    
        frame.render_widget(gauge, chunks[0]);
        frame.render_widget(p, chunks[1]);
    }

    fn process_input(&mut self, key: KeyCode) -> bool {
        
        // The function returns true when the app needs to terminate.

        match self.current_screen {

            CurrentScreen::Playlists => {
                match key {

                    KeyCode::Down => self.playlists_widget.next(),
                    KeyCode::Up => self.playlists_widget.previous(),
                    KeyCode::Enter => { 
                        if let Some(ind) = self.playlists_widget.get_selected() {
                            self.open_playlist(ind).unwrap();
                        }
                    },
                    KeyCode::Char('q') => return true,
                    _ => {}
                }
            },
            CurrentScreen::Songs => {

                match key {
                    
                    KeyCode::Down => {
                        self.songs_selmode = SelectionMode::Manual;
                        self.songs_widget.next();
                    },
                    KeyCode::Up => {
                        self.songs_selmode = SelectionMode::Manual;
                        self.songs_widget.previous();
                    },
                    KeyCode::Enter => {
                        if let Some(ind) = self.songs_widget.get_selected() {
                            self.play_ind(ind);
                            self.songs_widget.clear_filter();
                            self.songs_selmode = SelectionMode::Follow;
                        }
                    },
                    KeyCode::Left => { self.player.rewind(15); },
                    KeyCode::Right => { self.player.forward(15); },
                    KeyCode::Char(c) => {
                           
                        if self.songs_widget.is_filtered() { 
                            self.search_query.push(c);
                            self.songs_widget.filter(&self.search_query);
                        }
                        else { match c {

                            'p' => self.toggle_pause(),
                            'f' => self.toggle_follow(),
                            's' => {
                                self.search_query = String::new();
                                self.songs_widget.filter("");
                            },
                            'n' => self.play_next(),
                            'r' => self.toggle_shuffle(),
                            'q' => {
                                self.close_playlist();
                                // Terminate the app if it was playing a local playlist.
                                if self.dao.is_none() { return true; }
                            
                            },
                            'h' => { self.current_screen = CurrentScreen::SongControls; },
                            '+' => self.player.increase_volume(10),
                            '-' => self.player.decrease_volume(10),
                            c => {  
                                if let Some(digit) = c.to_digit(10) { 
                                    let pcent = digit as usize * 10;
                                    self.player.seek_percentage(pcent);
                                }
                            },
                        }}   
                    }, 
                    KeyCode::Backspace => {
                        if self.songs_widget.is_filtered() {
                            self.search_query.pop();
                            self.songs_widget.filter(&self.search_query);
                        }
                    },
                    KeyCode::Esc => self.songs_widget.clear_filter(),
                    _ => {}
                }
            },
            CurrentScreen::SongControls => { self.current_screen = CurrentScreen::Songs; } 
        }
        
        false
    }

    fn open_playlist(&mut self, ind: usize) -> Result<(), DbError> {
                 
        let playlist = self.playlists_widget.get_ind(ind);                            
        self.current_playlist = Some(playlist.title.clone());
        self.load_songs(playlist.id)?;
        
        self.current_screen = CurrentScreen::Songs;

        Ok(())
    }

    fn close_playlist(&mut self) {
        
        self.player.stop();
        self.current_song = None;
        self.current_song_ind = None;
        self.current_screen = CurrentScreen::Playlists;   
    }

    fn toggle_pause(&mut self) {
        
        if self.player.is_paused() { self.player.resume(); }
        else { self.player.pause() }
    }

    fn toggle_follow(&mut self) {
        
        if let (false, Some(ind)) = (self.songs_widget.is_filtered(), self.current_song_ind) {
            // Don't activate follow mode if the song list is filtered.
            self.songs_selmode = SelectionMode::Follow;
            self.songs_widget.select_ind(ind);
        }
    }

    fn toggle_shuffle(&mut self) {

        if let Some(ref mut playlist) = self.current_playlist {
            if self.shuffle {
                playlist.pop();
                playlist.pop();
                playlist.pop();
                self.shuffle = false;
            }
            else {
                playlist.push_str(" 🔀 ");
                self.shuffle = true;
            }

            if self.current_song_ind.is_none() { self.play_next(); }
        }  

    }

    fn play_next(&mut self) {
        
        let ind = self.current_song_ind.unwrap_or(0);
        let ind = 
            if self.shuffle {random::<usize>() % self.songs_widget.total_len()}
            else {(ind + 1) % self.songs_widget.total_len()};
   
        self.play_ind(ind);
    }

    fn play_ind(&mut self, ind: usize) {

        // Move the cursor if follow mode is active.
        if let SelectionMode::Follow = self.songs_selmode{ self.songs_widget.select_ind(ind); }
        
        self.player.stop();
        let song = self.songs_widget.get_ind(ind);       
        self.current_song = Some(song.clone());
        self.current_song_ind = Some(ind);

        let mut filename = song.title.replace(['/', '\\', ':', '*', '<', '>', '|', '\"'], "");
        filename.push_str(".mp3");

        let mut filepath = self.playlist_dir.clone();
        filepath.push(OsStr::new(&filename));

        if filepath.exists() { let _ = self.player.play_file(&filepath); }
        else if let Some(yt_id) = song.yt_id.as_ref() { 
            self.downloading = true;
            self.downloader.download_url(yt_id, &filepath);   
        }
    }
}