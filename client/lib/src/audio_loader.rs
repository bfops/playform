use hound;
use std;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(unused)]
pub enum SoundId {
  Rainforest,
  Footstep(u8),
}

impl SoundId {
  pub fn to_asset_path(&self) -> String {
    match *self {
      SoundId::Rainforest    => "Assets/rainforest_ambience-GlorySunz-1938133500.wav".to_owned(),
      SoundId::Footstep(idx) => format!("Assets/Walking_On_Gravel-SoundBible{}.wav", idx),
    }
  }
}

pub struct T {
  loaded: std::collections::HashMap<SoundId, Vec<f32>>,
}

pub fn new() -> T {
  T {
    loaded: std::collections::HashMap::new(),
  }
}

impl T {
  pub fn load(&mut self, id: SoundId) -> &Vec<f32> {
    self.loaded
      .entry(id)
      .or_insert_with(|| load_from_file(&id.to_asset_path()))
  }
}

fn load_from_file(path: &str) -> Vec<f32> {
  let mut reader = hound::WavReader::open(path).unwrap();
  reader.samples::<i16>()
  .map(|s| {
    s.unwrap() as f32 / 32768.0
  })
  .collect()
}
