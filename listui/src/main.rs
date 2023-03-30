mod widgets;
mod app;
mod utils;

use std::fs::create_dir_all;
use std::path::PathBuf;
use std::env;
use app::ListuiApp;
use argh::FromArgs;
use utils::{get_youtube_playlist, get_local_playlist};

#[derive(FromArgs)]
/// A simple music player for your terminal.
struct ListuiArgs {
    
    /// local directory or youtube playlist.
    #[argh(positional)]
    pub playlist: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let args: ListuiArgs = argh::from_env();

    // Directory where the database and .env file will be located.
    let mut data_dir = dirs::data_dir().expect("Failed to create data directory.");
    data_dir.push("listui");
    create_dir_all(data_dir.clone()).expect("Failed to create data directory.");

    let config_dir = dirs::config_dir();
    if let Some(mut config_dir) = config_dir {
        // Parse .env file. If it fails, default values will be used instead.
        config_dir.push("listui/listui.config");
        let _ = dotenvy::from_path(config_dir);
    }
    

    let database_path  =  (|| Some(PathBuf::from(env::var("DATABASE_PATH").ok()?)))()
        .unwrap_or_else(|| {
            data_dir.push("db.sqlite");
            data_dir
        });

    // Directory the downloaded music will the placed.
    let download_dir  =  (|| Some(PathBuf::from(env::var("DOWNLOAD_DIR").ok()?)))()
        .unwrap_or_else(|| {
            let mut dir = dirs::audio_dir().expect("Cannot find audio directory.");
            dir.push("listui");
            dir
        });

    let app: Option<ListuiApp> = {

        if let Some(arg) = args.playlist.as_ref() {
    
            let result = get_youtube_playlist(&database_path, arg);
            match result {
                Ok(Some(_)) => Some(ListuiApp::new_open_playlist(download_dir, database_path)?),
                Ok(None) => { 
                    let path = PathBuf::from(arg).canonicalize()?;
                    match get_local_playlist(&path) {
                        Some(tracks) => Some(ListuiApp::with_tracks(path, tracks)?),
                        None => None,
                    }
                },
                Err(e) => { 
                    eprintln!("{}", e); 
                    None
                }
            }
        }   
        else { Some(ListuiApp::new(download_dir, database_path)?) }
    };

    if let Some(mut app) = app { app.run()?; }
    else { eprintln!("Invalid argument.") }
    
   Ok(())
}