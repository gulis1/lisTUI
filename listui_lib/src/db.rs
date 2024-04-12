use diesel::SqliteConnection;
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind;
use diesel::result::Error as DieselError;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};


use std::cell::RefCell;
use std::path::Path;

use crate::models::*;
use crate::models::Playlist;
use crate::schema::track as TrackTable;
use crate::schema::playlist as PlaylistTable;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

#[derive(Debug)]
pub enum DbError {
    UnknownError,
    UniqueViolation,
    NotFoundError,
    ConnectionError,
    MigrationError
}

impl std::error::Error for DbError {}

impl std::fmt::Display for DbError {
    
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::UnknownError => write!(f, "An unknown error happened."),
            DbError::UniqueViolation => write!(f, "Playlist already present."),
            DbError::NotFoundError => write!(f, "Item not found."),
            DbError::ConnectionError => write!(f, "Failed to connect to database."),
            DbError::MigrationError =>  write!(f, "Failed to run database migrations."),
        }
    }
}

fn run_migrations(connection: &mut SqliteConnection) -> Result<(), DbError> {
    connection.run_pending_migrations(MIGRATIONS)
        .map_err(|_| DbError::MigrationError)?;

    Ok(())
}

// Struct used to communicate with the sqlite database.
pub struct Database {
    connection: RefCell<SqliteConnection>,
}

impl Database {

    /// Creates a new SQlite database connection.
    pub fn new(database_path: &Path) -> Result<Self, DbError> {
        
        let mut connection = SqliteConnection::establish(&database_path.as_os_str().to_string_lossy()).map_err(|_| DbError::ConnectionError)?;
        run_migrations(&mut connection)?;

        Ok(Database {
            connection: RefCell::new(connection)
        })
    }
    
    /// Gets all the playlists from the database.
    pub fn get_playlists(&self) -> Result<Vec<Playlist>, DbError> {

        PlaylistTable::table
            .load::<Playlist>(&mut*self.connection.borrow_mut())
        .map_err(convert_err)
    }

    /// Gets a playlist from the database given its ID.
    pub fn get_playlist(&self, playlist_id: i32) -> Result<Playlist, DbError> {

        PlaylistTable::table
            .filter(PlaylistTable::columns::id.is(playlist_id))
            .first::<Playlist>(&mut*self.connection.borrow_mut())
        .map_err(convert_err)
    }

    /// Saves a playlist into the database.
    pub fn save_playlist(&self, plist: NewPlaylist) -> Result<Playlist, DbError> {

        let result = diesel::insert_into(PlaylistTable::table)
            .values(&plist)
            .execute(&mut*self.connection.borrow_mut());


        result.and_then(|_| {

            PlaylistTable::table
                .filter(PlaylistTable::columns::yt_id.is(plist.yt_id))
                .first::<Playlist>(&mut*self.connection.borrow_mut())
        }).map_err(convert_err)
    }

    /// Deletes a playlist from the database.
    pub fn delete_playlist(&self, playlist_id: i32) -> Result<(), DbError> {

        
        let _: Result<usize, DieselError> = diesel::delete(TrackTable::table.filter(TrackTable::columns::playlist_id.is(playlist_id)))
            .execute(&mut*self.connection.borrow_mut());

        let result: Result<usize, DieselError> = diesel::delete(PlaylistTable::table.filter(PlaylistTable::columns::id.is(playlist_id)))
            .execute(&mut*self.connection.borrow_mut());

        match result {

            Ok(n) => {

                if n == 0 { Err(DbError::NotFoundError) }
                else { Ok(()) }       
            },
            Err(e) => Err(convert_err(e))
        }
    }

    /// Gets all tracks from a playlist.
    pub fn get_tracks(&self, playlist_id: i32) -> Result<Vec<Track>, DbError> {

        let result: Result<Vec<Track>, DieselError> = TrackTable::table
            .filter(TrackTable::columns::playlist_id.is(playlist_id))
            .load::<Track>(&mut*self.connection.borrow_mut());

        result.map_err(convert_err)

    }

    /// Saves new tracks for a playlist, without deleting previous ones.
    pub fn save_tracks(&self, mut videos: Vec<NewVideo>, playlist_id: i32) -> Result<(), DbError> {

        for vid in &mut videos {
            vid.playlist_id = Some(playlist_id);
        }

        diesel::insert_into(TrackTable::table)
            .values(videos)
            .execute(&mut*self.connection.borrow_mut())
            .map(|_| ()).map_err(convert_err)
    }

    /// Deletes all tracks from a playlist, and then saves the new ones.
    pub fn replace_tracks(&self, playlist_id: i32,  videos: Vec<NewVideo>) -> Result<(), DbError> {
        // Removes all tracks asociated with a playlists and inserts the new ones.

        diesel::delete(TrackTable::table.filter(TrackTable::columns::playlist_id.is(playlist_id)))
            .execute(&mut*self.connection.borrow_mut()).map_err(convert_err)?;

        self.save_tracks(videos, playlist_id)?;
        Ok(())
    }
}

fn convert_err(err: DieselError) -> DbError {

    match err {
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => DbError::UniqueViolation,
        _ => DbError::UnknownError   
    }
}



