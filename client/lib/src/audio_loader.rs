use hound;
use std;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(unused)]
pub enum SoundId {
  Rainforest,
  Footstep,
}

impl SoundId {
  pub fn to_asset_path(&self) -> &std::path::Path {
    match *self {
      SoundId::Rainforest => std::path::Path::new("Assets/rainforest_ambience-GlorySunz-1938133500.wav"),
      SoundId::Footstep   => std::path::Path::new("Assets/Walking_On_Gravel-SoundBible.wav"),
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

fn load_from_file(path: &std::path::Path) -> Vec<f32> {
  let mut reader = hound::WavReader::open(path.to_str().unwrap()).unwrap();
  reader.samples::<i16>()
  .map(|s| {
    s.unwrap() as f32 / 32768.0
  })
  .collect()
}
