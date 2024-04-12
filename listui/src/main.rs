mod widgets;
mod app;
mod utils;

use std::fs::File;
use std::{fs::create_dir_all, path::PathBuf};
use app::ListuiApp;
use argh::FromArgs;
use listui_lib::db::Database;
use simplelog::{Config, LevelFilter, WriteLogger};
use utils::{get_local_playlist, parse_playlist_url};

#[derive(FromArgs)]
/// A simple music player for your terminal.
struct ListuiArgs {
    
    /// local directory or youtube playlist.
    #[argh(positional)]
    pub playlist: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let args: ListuiArgs = argh::from_env();

    // Load config file.
    let config_path = utils::get_config_path().expect("Failed to get config path.");
    let _ = dotenvy::from_path(config_path);
    
    let log_path = utils::get_log_path().expect("Failed to get log path.");
    let _ = WriteLogger::init(LevelFilter::Info, Config::default(), File::create(log_path).unwrap());   

    let database_path = utils::get_db_path().expect("Failed to get database path.");
    let download_dir = utils::get_download_dir().expect("Failed to get download directory.");
    
    // Create directory to download all songs (If it does not exist).
    create_dir_all(&download_dir).expect("Failed to create download directory");

    let app: Option<ListuiApp> = {

        let dao = Database::new(&database_path)?;
        if let Some(arg) = args.playlist.as_ref() {
                        
            let playlist_ytid = parse_playlist_url(arg);
            match playlist_ytid {
                Some(yt_id) => Some(ListuiApp::new_open_playlist(download_dir, dao, yt_id)?),
                None => {

                    let path = PathBuf::from(arg).canonicalize()?;
                    match get_local_playlist(&path) {
                        Some(tracks) => Some(ListuiApp::with_tracks(path, tracks)?),
                        None => {
                            eprintln!("Directory not found.");
                            None
                        },
                    }
                }
            }
        }   
        else { Some(ListuiApp::new(download_dir, dao)?) }
    };

    if let Some(mut app) = app { app.run()?; }
       
   Ok(())
}
