pub fn leaves() -> String {
  struct Wave {
    freq: f32,
    amp: f32,
  }

  let waves = [
    Wave { freq:  8.0, amp: 1.0 },
    Wave { freq: 16.0, amp: 0.5 },
    Wave { freq:  2.0, amp: 0.8 },
  ];

  let mut contents = String::new();
  for wave in waves.iter() {
    contents.push_str(format!(r#"
    {{
      float freq = {};
      float amp = {};

      float dnoise = cnoise(freq * world_position);
      noise += sign(dnoise) * pow(abs(dnoise), 0.2) * amp;
      total_amp += amp;
    }}
    "#, wave.freq, wave.amp).as_str());
  }

  format!(r#"
    float total_amp = 0.0;
    float noise = 0.0;
    {}
    noise /= total_amp;

    return mix(vec3(0.1, 0.4, 0.0), vec3(0.3, 0.6, 0.0), noise);
  "#, contents)
}
