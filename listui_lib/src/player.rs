use std::fmt::Debug;
use std::sync::atomic::{AtomicI64, Ordering};
use std::{fs::File, time::Duration};
use std::io::BufReader;
use std::path::Path;
use rodio::{Decoder, OutputStream, Source, Sink};
use thiserror::Error;


#[derive(Error, Debug)]
pub enum PlayerError {
    #[error("Failed to create audio output stream: {0}")]
    StreamError(#[from] rodio::StreamError),
    #[error("I/O error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Decoding error: {0}")]
    DecodingError(#[from] rodio::decoder::DecoderError),
}

pub struct Player {

    sink: Sink,
    current_track_duration: AtomicI64
}

impl Debug for Player {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl Player {

    pub fn try_default() -> Result<Self, PlayerError> {
        
        let (stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle).unwrap();
        std::mem::forget(stream);
        Ok(Self {
            sink,
            current_track_duration: AtomicI64::new(0)
        })
    }

    pub fn play_file(&self, path: &Path) -> Result<(), PlayerError> {
        
        let file = BufReader::new(File::open(path)?);
        let source = Decoder::new(file)?;
        self.stop();
        self.current_track_duration.store(source.total_duration().unwrap().as_secs() as i64, Ordering::SeqCst);

        
        self.sink.append(source);
        Ok(())
    }

    pub fn is_playing(&self) -> bool {
        // WARNING
        self.current_track_duration.load(Ordering::Relaxed) >= 0
    }

    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }

    pub fn seek(&self, seconds: u64) {
        self.sink.try_seek(Duration::from_secs(seconds)).expect("Failed to seek");
    }

    pub fn seek_percentage(&self, percentage: u64) {
        
        if let Some(duration) = self.get_duration() {
            let time = percentage * duration / 100;
            self.sink.try_seek(Duration::from_secs(time)).expect("Failed to seek");
        }
        
    }

    pub fn forward(&self, seconds: u64) {
        
        if let Some(progress) = self.get_progress() {
            let newpos = progress + seconds;
            let duration = self.current_track_duration.load(Ordering::Relaxed);
            self.sink.try_seek(Duration::from_secs(newpos)).expect("Failed to seek");
            if newpos + seconds >= duration as u64 {
                self.stop();
            }
        }
    }

    pub fn rewind(&self, seconds: u64) {
    
        if let Some(progress) = self.get_progress() {
            if progress > seconds {
                let newpos = progress - seconds;
                self.sink.try_seek(Duration::from_secs(newpos)).expect("Failed to seek");
            }
            else { 
                self.sink.try_seek(Duration::from_secs(0)).expect("Failed to seek");
            }
        }
    }

    pub fn pause(&self) {
        self.sink.pause();
    }

    pub fn resume(&self) {
        self.sink.play();    
    }

    pub fn get_progress(&self) -> Option<u64> {
    
        if !self.is_playing() {
            None
        }
        else {
            Some(self.sink.get_pos().as_secs())
        }
    }

    pub fn get_duration(&self) -> Option<u64> {

        if !self.is_playing() {
            None
        }
        else {
            Some(self.current_track_duration.load(Ordering::SeqCst) as u64)
        }
    }

    pub fn increase_volume(&self, volume_inc: i32) {

        let mut new_volume = self.sink.volume() + (volume_inc as f32 / 100.0);
        if new_volume > 2.0 { new_volume = 2.0; } 
        self.sink.set_volume(new_volume);
    }

    pub fn decrease_volume(&self, volume_inc: i32) {

        let mut new_volume = self.sink.volume() - (volume_inc as f32 / 100.0);
        if new_volume < 0.0 { new_volume = 0.0; }
        self.sink.set_volume(new_volume);
    }

    pub fn get_volume(&self) -> i32 {
        (100.0 * self.sink.volume()).round() as i32
    }

    pub fn stop(&self) {
        self.current_track_duration.store(-1, Ordering::SeqCst);
        self.sink.stop();
    }
}
