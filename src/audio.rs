use std::time;

use sdl2::{
  audio::{AudioQueue, AudioSpecDesired},
  Sdl,
};

use crate::apu;

pub struct Audio(Box<dyn Fn(&[f32])>);

impl Audio {
  pub fn new(sdl: &Sdl) -> Audio {
    let audio = sdl
      .audio()
      .expect("failed to initialize SDL audio subsystem");
    let audio_queue: AudioQueue<f32> = audio.open_queue(None, 
      &AudioSpecDesired {
        freq: Some(apu::SAMPLE_RATE as i32),
        channels: Some(2),
        samples: Some(apu::SAMPLES as u16 * 2),
      }
    ).expect("failed to create audio queue");
    audio_queue.resume();
    Self(
      Box::new(move |buffer| {
        while audio_queue.size() > 1024 * 4 * 2 {
          std::thread::sleep(time::Duration::from_millis(1));
        }
        audio_queue.queue_audio(buffer).unwrap();
      })
    )
  }
  pub fn play(&self, buffer: &[f32]) {
    self.0(buffer);
  }
}
