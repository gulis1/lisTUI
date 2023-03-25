CREATE TABLE track (

    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    title TEXT NOT NULL,
    yt_id TEXT,
    playlist_id INTEGER,
    FOREIGN KEY(playlist_id) REFERENCES playlist(id) ON DELETE CASCADE
)