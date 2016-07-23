use portaudio;
use std;
use std::sync::{Mutex};

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

  let mut tracks_playing = audio::TracksPlaying::new(channels * buffer_size);

  let portaudio = portaudio::PortAudio::new().unwrap();
  let params = portaudio.default_output_stream_params(channels as i32).unwrap();
  let settings = portaudio::OutputStreamSettings::new(params, sample_rate, buffer_size as u32);

  let callback = {
    let tracks_playing: *mut audio::TracksPlaying = &mut tracks_playing as *mut _ as *mut _;
    let tracks_playing: &mut audio::TracksPlaying = unsafe { &mut *tracks_playing };
    move |portaudio::OutputStreamCallbackArgs { buffer, .. }| {
      for x in buffer.iter_mut() {
        *x = 0.0;
      }
      tracks_playing.with_buffer(|b| {
        assert!(b.len() == buffer.len());
        for (i, x) in buffer.iter_mut().enumerate() {
          *x = b[i];
        }
      });
      portaudio::StreamCallbackResult::Continue
    }
  };

  let mut stream = portaudio.open_non_blocking_stream(settings, callback).unwrap();
  stream.start().unwrap();

  let mut audio_loader = audio_loader::new();

  while !*quit.lock().unwrap() && stream.is_active() == Ok(true) {
    if let Some(up) = recv_message() {
      match up {
        Message::PlayLoop(id) => {
          tracks_playing.push(audio::Track::new(audio_loader.load(id).clone(), true))
        },
        Message::PlayOneShot(id) => {
          tracks_playing.push(audio::Track::new(audio_loader.load(id).clone(), false))
        },
      }
    } else {
      std::thread::sleep(std::time::Duration::from_millis(1));
    }

    tracks_playing.refresh_buffer();
  }

  stream.stop().unwrap();
  stream.close().unwrap();
}
