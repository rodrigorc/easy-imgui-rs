use std::time::{Duration, Instant};

/// This struct handles the main loop going to idle when there is no user input for a while.
pub struct Idler {
    idle_time: Duration,
    idle_frame_count: u32,
    last_input_time: Instant,
    last_input_frame: u32,
}

impl Default for Idler {
    fn default() -> Idler {
        let now = Instant::now();
        Idler {
            idle_time: Duration::from_secs(1),
            idle_frame_count: 60,
            last_input_time: now,
            last_input_frame: 0,
        }
    }
}

impl Idler {
    /// Sets the maximum time that the window will be rendered without user input.
    pub fn set_idle_time(&mut self, time: Duration) {
        self.idle_time = time;
    }
    /// Sets the maximum number of frames time that the window will be rendered without user input.
    pub fn set_idle_frame_count(&mut self, frame_count: u32) {
        self.idle_frame_count = frame_count;
    }
    /// Call this when the window is renderer.
    pub fn incr_frame(&mut self) {
        // An u32 incrementing 60 values/second would overflow after about 2 years, better safe
        // than sorry.
        self.last_input_frame = self.last_input_frame.saturating_add(1);
    }
    /// Check whether the window should go to idle or keep on rendering.
    pub fn has_to_render(&self) -> bool {
        self.last_input_frame < self.idle_frame_count
            || Instant::now().duration_since(self.last_input_time) < self.idle_time
    }
    /// Notify this struct that user input happened.
    pub fn ping_user_input(&mut self) {
        self.last_input_time = Instant::now();
        self.last_input_frame = 0;
    }
}
