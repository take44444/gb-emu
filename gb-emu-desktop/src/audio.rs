use std::time;

use sdl2::{
  audio::{AudioQueue, AudioSpecDesired},
  Sdl,
};

use gbemu::{
  SAMPLES,
  SAMPLE_RATE,
};

pub struct Audio(pub Box<dyn Fn(&[f32])>);

impl Audio {
  pub fn new(sdl: &Sdl) -> Audio {
    let audio = sdl
      .audio()
      .expect("failed to initialize SDL audio subsystem");
    let audio_queue: AudioQueue<f32> = audio.open_queue(None, 
      &AudioSpecDesired {
        freq: Some(SAMPLE_RATE as i32),
        channels: Some(2),
        samples: Some(SAMPLES as u16 * 2),
      }
    ).expect("failed to create audio queue");
    audio_queue.resume();
    Self(
      Box::new(move |buffer| {
        while audio_queue.size() > 8192 {
            std::thread::sleep(time::Duration::from_millis(1));
        }
        audio_queue.queue_audio(buffer).unwrap();
      }),
    )
  }
}
