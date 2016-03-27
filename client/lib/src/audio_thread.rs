use hound;
use portaudio;
use std;
use std::sync::{Mutex};

use audio;

pub fn audio_thread(
  quit: &Mutex<bool>,
) {
  let sample_rate = 44100.0;
  let channels = 2;
  let buffer_size = 1 << 10;

  let mut tracks_playing = audio::TracksPlaying::new(buffer_size);

  let portaudio = portaudio::PortAudio::new().unwrap();
  let params = portaudio.default_output_stream_params(channels).unwrap();
  let settings = portaudio::OutputStreamSettings::new(params, sample_rate, buffer_size as u32);

  let callback = {
    let tracks_playing: *mut audio::TracksPlaying = unsafe { std::mem::transmute(&mut tracks_playing) };
    let tracks_playing: &mut audio::TracksPlaying = unsafe { std::mem::transmute(tracks_playing) };
    move |portaudio::OutputStreamCallbackArgs { buffer, .. }| {
      for x in buffer.iter_mut() {
        *x = 0.0;
      }
      tracks_playing.with_buffer(|b| {
        assert!(2 * b.len() == buffer.len());
        for (i, x) in buffer.iter_mut().enumerate() {
          *x = b[i / 2];
        }
      });
      portaudio::StreamCallbackResult::Continue
    }
  };

  let mut stream = portaudio.open_non_blocking_stream(settings, callback).unwrap();
  stream.start().unwrap();

  let ambient_track = load_ambient_track();
  tracks_playing.push(ambient_track);

  while !*quit.lock().unwrap() && stream.is_active() == Ok(true) {
    tracks_playing.refresh_buffer();
    std::thread::sleep(std::time::Duration::from_millis(10));
  }

  stream.stop().unwrap();
  stream.close().unwrap();
}

fn load_ambient_track() -> audio::Track {
  let mut reader = hound::WavReader::open("Assets/rainforest_ambience-GlorySunz-1938133500.wav").unwrap();
  let data: Vec<f32> =
    reader.samples::<i16>()
    .map(|s| {
      s.unwrap() as f32 / 32768.0
    })
    .collect();
  audio::Track::new(data)
}
