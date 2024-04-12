use std::cell::RefCell;
use std::path::Path;
use soloud::*;

#[derive(Debug)]
pub struct WavWrapper(pub Wav);

unsafe impl Send for WavWrapper {}
impl Default for WavWrapper {
    fn default() -> Self {
        Self(Wav::default())
    }
}


#[derive(Debug)]
pub struct Player {

    sl: RefCell<Soloud>,
    current_handle: RefCell<Option<Handle>>,
    wav: RefCell<WavWrapper>

}

impl Default for Player {

    fn default() -> Self {

        Self {
            sl: RefCell::new(Soloud::default().expect("Unable to initialize soloud engine.")),
            current_handle: RefCell::new(None),
            wav: RefCell:: new(WavWrapper::default())
        }
    }
}


impl Player {

    pub fn play_file(&self, path: &Path) -> Result<(), SoloudError> {
        
        let wav = Wav::from_path(path)?;
        let handle = self.sl.borrow_mut().play(&wav);
        self.current_handle.replace(Some(handle));
        self.wav.replace(WavWrapper(wav));

        Ok(())
    }

    pub fn play_wav(&self, wav: WavWrapper) -> Result<(), SoloudError> {

        let handle = self.sl.borrow_mut().play(&wav.0);
        self.current_handle.replace(Some(handle));
        self.wav.replace(wav);

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

    pub fn seek(&self, seconds: u64) {

        let handle = self.current_handle.borrow();
        if let Some(handle) = *handle {
            self.sl.borrow().seek(handle, seconds as f64).expect("Seek error.");
        }
    }

    pub fn seek_percentage(&self, percentage: u64) {

        let handle = self.current_handle.borrow();
        if let Some(handle) = *handle {
            let time = (percentage as f64 / 100.0) * self.get_duration() as f64;
            self.sl.borrow().seek(handle, time).expect("Seek error.");
        }
    }

    pub fn forward(&self, seconds: u64) {

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

    pub fn rewind(&self, seconds: u64) {

        let handle = *self.current_handle.borrow();
        let progress = self.get_progress();

        if let (Some(handle), Some(progress)) = (handle, progress) {
            
            let sl = self.sl.borrow();

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

    pub fn get_progress(&self) -> Option<u64> {

        let handle = self.current_handle.borrow();
        handle.map(|h| self.sl.borrow().stream_position(h) as u64)
    }

    pub fn get_duration(&self) -> u64 {
        self.wav.borrow().0.length() as u64
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
