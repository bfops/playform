pub fn dirt() -> String {
  struct Wave {
    freq: f32,
    amp: f32,
  }

  let mut waves = Vec::new();
  for i in 0..10 {
    waves.push(
      Wave { freq: (1 << i) as f32, amp: 1.0 / (1 << i) as f32 }
    );
  }

  let mut contents = String::new();
  for wave in waves.iter() {
    contents.push_str(format!(r#"
    {{
      float freq = {};
      float amp = {};

      float dnoise = abs(cnoise(freq * world_position));
      noise += dnoise * amp;
      total_amp += amp;
    }}
    "#, wave.freq, wave.amp).as_str());
  }

  format!(r#"
  float total_amp = 0.0;
  float noise = 0.0;
  {}
  noise /= total_amp;
  return mix(vec3(0.4, 0.3, 0.1), vec3(0.7, 0.6, 0.4), noise);
  "#, contents)
}
