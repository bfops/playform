use portaudio;
use std;
use std::sync::{Mutex};
use time;

use audio;
use audio_loader;

#[allow(unused)]
pub enum Message {
  PlayLoop(audio_loader::SoundId),
  PlayOneShot(audio_loader::SoundId),
}

pub fn audio_thread<RecvMessage>(
  quit: &Mutex<bool>,
  recv_message: &mut RecvMessage,
) where
  RecvMessage: FnMut() -> Option<Message>,
{
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

  let mut audio_loader = audio_loader::new();

  while !*quit.lock().unwrap() && stream.is_active() == Ok(true) {
    let start = time::precise_time_ns();
    let mut i = 0;
    while let Some(up) = recv_message() {
      match up {
        Message::PlayLoop(id) => {
          tracks_playing.push(audio::Track::new(audio_loader.load(id).clone(), true))
        },
        Message::PlayOneShot(id) => {
          tracks_playing.push(audio::Track::new(audio_loader.load(id).clone(), false))
        },
      }

      if i > 10 {
        i -= 10;
        if time::precise_time_ns() - start >= 1_000_000 {
          break
        }
      }
      i += 1;
    }

    tracks_playing.refresh_buffer();
    std::thread::sleep(std::time::Duration::from_millis(1));
  }

  stream.stop().unwrap();
  stream.close().unwrap();
}
