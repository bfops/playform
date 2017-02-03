//! Load audio assets into memory

use hound;

use common::fnv_map;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(missing_docs)]
pub enum SoundId {
  #[allow(missing_docs)]
  Rainforest,
  #[allow(missing_docs)]
  Footstep(u8),
}

impl SoundId {
  #[allow(missing_docs)]
  pub fn to_asset_path(&self) -> String {
    match *self {
      SoundId::Rainforest    => "sounds/rainforest_ambience-GlorySunz-1938133500.wav".to_owned(),
      SoundId::Footstep(idx) => format!("sounds/Walking_On_Gravel-SoundBible{}.wav", idx),
    }
  }
}

#[allow(missing_docs)]
pub struct T {
  loaded: fnv_map::T<SoundId, Vec<f32>>,
}

#[allow(missing_docs)]
pub fn new() -> T {
  T {
    loaded: fnv_map::new(),
  }
}

impl T {
  #[allow(missing_docs)]
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
