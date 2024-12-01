# 0.2.4

### New features:

- Now the loading screen that appears when fetching a playlist displays the number of fetched videos.

### Improvements:

- Changed the audio library to rodio. Much faster song loading times.
- Added logs (by default, they are stored in a file in the same path as the database). This will hopefully be very useful to debug future bugs.

### Bug fixes:

- Search did not work when using caps.

# 0.2.3

### Bug fixes:

- App failed to create the direcotry to place downloaded youtube songs.

# 0.2.2

### Bug fixes:
- Fixed relative paths again.
- Skipping forward when a song was loading caused it to end early.

### New features:
- Added volume controls to the help menu.
- Progress bar now shows playing/paused symbols.

# 0.2.1

### Bug fixes:

- Relative paths didn't work when using local playlists.
- Local tracks with illegal Windows filenames now work properly on Linux.
- A song would still begin playing after being downloaded even if the playlists was closed.

# 0.2.0

### New features:

- Playlists can now be updated and deleted from the playlists menu.
- The app now displays a loading animation when it's fetching a playlist.

### Improvements:

- Some internal changes to how the player works.

# 0.1.3

### Bug fixes:

- Player got stuck when skipping a song that was being downloaded.
- First song of a playlist was not being fetched when using Invidious API.

# 0.1.2

### Bug fixes:

- Shuffle icon didn't show up.

### New features:

- Pressing 'b' will now play the previous song.

### Improvements:

- Changed how the shuffling system works. It now shows the order the shuffled songs will be played.

# 0.1.1

### Improvements:

- The application now shows an error if yt-dlp and ffmpeg are not installed when selecting a YouTube playlist.
