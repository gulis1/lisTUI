# 0.2.2

- Bug fix: fixed relative paths again.
- Bug fix: skipping forward when a song was loading caused it to end early.
- Added volume controls to the help menu.
- Progress bar now shows playing/paused symbols.

# 0.2.1

- Bug fix: relative paths didn't work when using local playlists.
- Bug fix: local tracks with illegal Windows filenames now work properly on Linux.
- Bug fix: a song would still begin playing after being downloaded even if the playlists was closed.

# 0.2.0

- Playlists can now be updated and deleted from the playlists menu.
- The app now displays a loading animation when it's fetching a playlist.
- Some internal changes to how the player works.

# 0.1.3

- Bug fix: player got stuck when skipping a song that was being downloaded.
- Bug fix: first song of a playlist was not being fetched uwhen using Invidious API.

# 0.1.2

- Changed how the shuffling system works. It now shows the order the shuffled songs will be played.
- Pressing 'b' will now play the previous song.
- Bug fix: shuffle icon didn't show up.

# 0.1.1

- The application now shows an error if yt-dlp and ffmpeg are not installed when selecting a YouTube playlist.