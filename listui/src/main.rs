mod widgets;
mod app;
mod utils;

use std::{fs::create_dir_all, path::PathBuf};
use std::env;
use app::ListuiApp;
use argh::FromArgs;
use listui_lib::db::Dao;
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
    
    let database_path  =  (|| PathBuf::from(env::var("DATABASE_PATH").ok()?).canonicalize().ok())()
        .unwrap_or_else(|| {
            data_dir.push("db.sqlite");
            data_dir
        });

    // Directory the downloaded music will the placed.
    let download_dir  =  (|| PathBuf::from(env::var("DOWNLOAD_DIR").ok()?).canonicalize().ok())()
        .unwrap_or_else(|| {
            let mut dir = dirs::audio_dir().expect("Cannot find audio directory.");
            dir.push("listui");
            dir
        }).canonicalize()?;

    let app: Option<ListuiApp> = {

        let dao = Dao::new(&database_path)?;
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
