use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct FrameBuffer {
    width: u32,
    height: u32,
    pixels: Vec<u32>,
}

pub type SharedFrameBuffer = Arc<Mutex<FrameBuffer>>;

impl FrameBuffer {
    pub fn new_shared() -> SharedFrameBuffer {
        Arc::new(Mutex::new(FrameBuffer {
            width: 0,
            height: 0,
            pixels: vec![],
        }))
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    pub fn pixels_mut(&mut self) -> &mut [u32] {
        &mut self.pixels
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.pixels.resize(width as usize * height as usize, 0);
    }
}
