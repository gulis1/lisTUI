use listui_lib::db::{Dao, DbError};

use listui_lib::models::{Playlist, Track};

use tokio::sync::mpsc;
use tui::Frame;
use tui::Terminal;
use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Direction, Layout, Rect};

use std::error::Error;
use std::io::Stdout;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossterm::event;
use crossterm::event::{Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};

use crate::widgets::*;
use crate::widgets::list::ListWidget;
use crate::widgets::player::PlayerWidget;
use crate::utils;

#[derive(Clone, Copy)]
pub enum CurrentScreen {

    Playlists,
    Songs,
    SongControls,
    YtDlpError
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
    player_widget: PlayerWidget,
    recv: mpsc::Receiver<utils::Message>,

    dao: Option<Dao>,

    current_playlist: Option<String>,
    current_song_ind: Option<usize>,
    songs_selmode: SelectionMode,

    search_query: String,
    downloading: bool
}

impl ListuiApp {

    pub fn new(playlist_dir: PathBuf, dao: Dao) -> Result<Self, Box<dyn std::error::Error>> {

        let (sender, recv) = mpsc::channel::<utils::Message>(5);
        Ok(Self {
           
            current_screen: CurrentScreen::Playlists,
            playlists_widget: ListWidget::with_items("Playlists", dao.get_playlists()?),
            songs_widget: ListWidget::empty("..."),
            player_widget: PlayerWidget::new(&playlist_dir, sender, 3),
            recv,
            
            dao: Some(dao),

            current_playlist: None,
            current_song_ind: None,
            songs_selmode: SelectionMode::Follow,
            search_query: String::new(),
            downloading: false
        })
    }

    pub fn new_open_playlist(playlist_dir: PathBuf, dao: Dao) -> Result<Self, Box<dyn std::error::Error>> {

        let mut app = ListuiApp::new(playlist_dir, dao)?;
        app.playlists_widget.select_ind(app.playlists_widget.total_len() - 1);
        
        Ok(app)
    }

    pub fn with_tracks(playlist_dir: PathBuf, tracks: Vec<Track>) -> Result<Self, Box<dyn std::error::Error>> {

        let playlist_name = (|| {
            Some(playlist_dir.file_name()?.to_string_lossy().to_string())
        })().unwrap_or(String::from("Unknown playlist."));

        let (sender, recv) = mpsc::channel::<utils::Message>(5);
        Ok(Self {
           
            current_screen: CurrentScreen::Songs,
            playlists_widget: ListWidget::empty("Playlists"),
            songs_widget: ListWidget::with_items("Playlists", tracks),
            player_widget: PlayerWidget::new(&playlist_dir, sender, 3),
            recv,
            
            dao: None,

            current_playlist: Some(playlist_name),
            current_song_ind: None,
            songs_selmode: SelectionMode::Follow,
            search_query: String::new(),
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
        
        match self.recv.try_recv() {
            Ok(utils::Message::SongFinished) => self.play_next(), 
            Err(_) => {}
        }
    }
    
    fn load_songs(&mut self, playlist_id: i32) -> Result<(), DbError> {
        
        // Loads from the DB all tracks of the playlist with the given id.
        if let Some(ref dao) = self.dao {
            let playlist = dao.get_playlist(playlist_id)?;
            let songs = dao.get_tracks(playlist_id)?;
            self.songs_widget = ListWidget::with_items(&playlist.title, songs);
            self.current_playlist = Some(playlist.title);
    
            Ok(())
        }
        else { Err(DbError::ConnectionError) }
    }

    fn draw(&mut self, frame: &mut Frame<CrosstermBackend<Stdout>>) {
        
        if frame.size().width < 25{ draw_error_msg(frame, "-->(x_x)<--"); }
        else if frame.size().height < 10 { draw_error_msg(frame,"Please make the terminal a bit taller :(") }
        else { match self.current_screen {

            CurrentScreen::Playlists => self.draw_playlists(frame, frame.size()),
            CurrentScreen::Songs => self.draw_songs(frame, frame.size()),
            CurrentScreen::SongControls => draw_controls_screen(frame, frame.size()),
            CurrentScreen::YtDlpError => draw_error_msg(frame, "Please install yt-dlp and ffmpeg first.")
        }}  
    }    

    fn draw_playlists(&mut self, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {

        if area.height < 20 || area.width < 50 {
            self.playlists_widget.draw(frame, area);
        }
        else {

            let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(10), Constraint::Length(area.height - 10)].as_ref())
            .split(area);

            draw_logo(frame, chunks[0]);
            self.playlists_widget.draw(frame, chunks[1]);
        }
    }

    fn draw_songs(&mut self, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {

        let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(area.height - 5), Constraint::Length(5)].as_ref())
                .split(area);

        self.songs_widget.draw(frame, chunks[0]);   
        self.player_widget.draw(frame, chunks[1]);
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
                    KeyCode::Char('d') => {
                        if let Some(ind) = self.playlists_widget.get_selected() {
                            self.delete_playlist(ind);
                        }
                    }
                    KeyCode::Char('u') => {
                        if let Some(ind) = self.playlists_widget.get_selected() {
                            self.update_playlist(ind);
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
                            self.activate_follow();
                        }
                    },
                    KeyCode::Left => { self.player_widget.rewind(15); },
                    KeyCode::Right => { self.player_widget.forward(15); },
                    KeyCode::Char(c) => {
                           
                        if self.songs_widget.is_filtered() { 
                            self.search_query.push(c);
                            self.songs_widget.filter(&self.search_query);
                        }
                        else { match c {

                            'p' => self.player_widget.toggle_pause(), 
                            'f' => self.activate_follow(),
                            's' => {
                                self.search_query = String::new();
                                self.songs_widget.filter("");
                            },
                            'n' => self.play_next(),
                            'b' => self.play_previous(),
                            'r' => {
                                self.stop_playing();
                                self.songs_widget.toggle_shuffle();
                            },
                            'q' => {
                                self.close_playlist();
                                // Terminate the app if it was playing a local playlist.
                                if self.dao.is_none() { return true; }
                            
                            },
                            'h' => { self.current_screen = CurrentScreen::SongControls; },
                            '+' => self.player_widget.increase_volume(10),
                            '-' => self.player_widget.decrease_volume(10),
                            c => {  
                                if let Some(digit) = c.to_digit(10) { 
                                    let pcent = digit as u64 * 10;
                                    self.player_widget.seek_percentage(pcent);
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
            CurrentScreen::SongControls => { self.current_screen = CurrentScreen::Songs; },
            CurrentScreen::YtDlpError => { self.current_screen = CurrentScreen::Playlists; }
        }
        
        false
    }

    fn open_playlist(&mut self, ind: usize) -> Result<(), DbError> {
        
        if utils::probe_ytdlp() && utils::probe_ffmpeg() {
            let playlist = self.playlists_widget.get_ind(ind);                            
            self.load_songs(playlist.id)?;
            self.current_screen = CurrentScreen::Songs;
        }
        else { self.current_screen = CurrentScreen::YtDlpError; }

        Ok(())
    }

    fn delete_playlist(&mut self, ind: usize) {
        if let Some(dao) = self.dao.as_ref() {
            dao.delete_playlist(self.playlists_widget.get_ind(ind).id).expect("Failed to delete playlist.");
            let playlists = dao.get_playlists().expect("Failed to get playlists.");
            self.playlists_widget = ListWidget::with_items("Playlists", playlists);
        }
    }

    fn update_playlist(&mut self, ind: usize) {
        if let Some(dao) = self.dao.as_ref() {
            
            let playlist = self.playlists_widget.get_ind(ind);
            let (_, videos) = utils::get_youtube_playlist(&playlist.yt_id, false).expect("Failed to fetch playlist info.",);
            dao.replace_tracks(playlist.id, videos).expect("Failed to update videos.");
            self.playlists_widget = ListWidget::with_items("Playlists", dao.get_playlists().expect("Failed to get playlists."));
        }
    }

    fn close_playlist(&mut self) {
        
        self.stop_playing();
        self.current_screen = CurrentScreen::Playlists;   
    }

    fn activate_follow(&mut self) {
        
        if let (false, Some(ind)) = (self.songs_widget.is_filtered(), self.current_song_ind) {
            // Don't activate follow mode if the song list is filtered.
            self.songs_selmode = SelectionMode::Follow;
            self.songs_widget.select_ind(ind);
        }
    }

    fn play_previous(&mut self) {
        
        self.downloading = false;
        let ind = match self.current_song_ind {
            Some(ind) => (ind -1 ) % self.songs_widget.total_len(),
            None => 0,
        };
        
        self.play_ind(ind);
    }

    fn play_next(&mut self) {
        
        self.downloading = false;
        let ind = match self.current_song_ind {
            Some(ind) => (ind + 1) % self.songs_widget.total_len(),
            None => 0,
        };
        
        self.play_ind(ind);
    }

    fn stop_playing(&mut self) {
        self.player_widget.stop();
        self.current_song_ind = None;
    }

    fn play_ind(&mut self, ind: usize) {

        // Move the cursor if follow mode is active.
        if let SelectionMode::Follow = self.songs_selmode{ self.songs_widget.select_ind(ind); }
        
        let song = self.songs_widget.get_ind(ind);       
        self.current_song_ind = Some(ind);
        self.player_widget.play(song.clone());
    }
}
