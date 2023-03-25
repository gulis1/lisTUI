use std::cell::RefCell;
use std::path::Path;
use soloud::*;

pub struct Player {

    sl: RefCell<Soloud>,
    current_handle: RefCell<Option<Handle>>,
    wav: RefCell<Wav>,
}

impl Default for Player {

    fn default() -> Self {

        Self {
            sl: RefCell::new(Soloud::default().expect("Unable to initialize soloud engine.")),
            current_handle: RefCell::new(None),
            wav: RefCell::new(audio::Wav::default())
        }
    }
}


impl Player {

    pub fn play_file(&self, path: &Path) -> Result<(), SoloudError> {
        
        self.wav.borrow_mut().load(path)?;
        let handle = self.sl.borrow_mut().play(&*self.wav.borrow());
        self.current_handle.replace(Some(handle));

        Ok(())
    }

    pub fn is_playing(&self) -> bool {
        
        let handle = self.current_handle.borrow();
        match *handle {
            Some(h) => self.sl.borrow().is_valid_voice_handle(h),
            None => false
        }
    }

    pub fn is_paused(&self) -> bool {
        
        let handle = self.current_handle.borrow();
        match *handle {
            Some(h) => self.sl.borrow().pause(h),
            None => false,
        } 
    }

    pub fn seek(&self, seconds: usize) {

        let handle = self.current_handle.borrow();
        if let Some(handle) = *handle {
            self.sl.borrow().seek(handle, seconds as f64).expect("Seek error.");
        }
    }

    pub fn seek_percentage(&self, percentage: usize) {

        let handle = self.current_handle.borrow();
        if let Some(handle) = *handle {
            let time = (percentage as f64 / 100.0) * self.get_duration() as f64;
            self.sl.borrow().seek(handle, time).expect("Seek error.");
        }
    }

    pub fn forward(&self, seconds: usize) {

        let handle = *self.current_handle.borrow();
        let progress = self.get_progress();

        if let (Some(handle), Some(progress)) = (handle, progress) {

            let sl = self.sl.borrow();
            let newpos = progress + seconds;

            if newpos < self.get_duration() {
                sl.seek(handle, newpos as f64).expect("Seek error.");
            }
            else { self.stop(); }
        }
    }

    pub fn rewind(&self, seconds: usize) {

        let handle = *self.current_handle.borrow();
        let progress = self.get_progress();

        if let (Some(handle), Some(progress)) = (handle, progress) {
            
            let sl = self.sl.borrow();
            let progress = progress;

            if progress > seconds {
                let newpos = progress - seconds;
                sl.seek(handle, newpos as f64).expect("Seek error.");
            }

            else { 
                sl.seek(handle, 0.0).expect("Seek error.");
            }
        }
    }

    pub fn pause(&self) {
        
        let handle = self.current_handle.borrow();
        if let Some(handle) = *handle {
            self.sl.borrow_mut().set_pause(handle, true);
        }
    }

    pub fn resume(&self) {
        
        let handle = self.current_handle.borrow();
        if let Some(handle) = *handle {
            self.sl.borrow_mut().set_pause(handle, false);
        }
    }

    pub fn get_progress(&self) -> Option<usize> {

        let handle = self.current_handle.borrow();
        handle.map(|h| self.sl.borrow().stream_position(h) as usize)
    }

    pub fn get_duration(&self) -> usize {
        self.wav.borrow().length() as usize
    }

    pub fn set_repeat(&self, value: bool) {

        let handle = self.current_handle.borrow();
        if let Some(handle) = *handle {
            self.sl.borrow_mut().set_looping(handle, value);
        }
    }

    pub fn increase_volume(&self, volume_inc: i32) {

        let mut sl =  self.sl.borrow_mut();
        
        let mut new_volume = sl.global_volume() + (volume_inc as f32 / 100.0);
        if new_volume > 3.0 { new_volume = 3.0; } 
        sl.set_global_volume(new_volume);
    }

    pub fn decrease_volume(&self, volume_inc: i32) {

        let mut sl =  self.sl.borrow_mut();
        
        let mut new_volume = sl.global_volume() - (volume_inc as f32 / 100.0);
        if new_volume < 0.0 { new_volume = 0.0; }
        sl.set_global_volume(new_volume);
    }

    pub fn get_volume(&self) -> i32 {
        (100.0 * self.sl.borrow().global_volume()).round() as i32
    }

    pub fn stop(&self) {
        let mut handle = self.current_handle.borrow_mut();
        if let Some(handle) = *handle {
            self.sl.borrow().stop(handle);
        }

        handle.take();
    }
}