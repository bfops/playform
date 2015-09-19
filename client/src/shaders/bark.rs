pub fn bark() -> String {
  struct Wave {
    freq: f32,
    amp: f32,
  }

  let waves = [
    Wave { freq:  64.0  / 1000.0, amp: 1.0 },
    Wave { freq: 128.0  / 1000.0, amp: 0.6 },
    Wave { freq: 256.0  / 1000.0, amp: 0.4 },
    Wave { freq: 1024.0 / 1000.0, amp: 0.4 },
    Wave { freq: 2048.0 / 1000.0, amp: 0.4 },
  ];

  let mut contents = String::new();
  for wave in &waves {
    contents.push_str(format!(r#"
    {{
      float freq = {};
      float amp = {};

      float dnoise = cnoise(freq * world_position);
      // sharpen
      dnoise = sign(dnoise) * pow(abs(dnoise), 0.2);
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
    noise = (noise + 1) / 2;
    float lerp = 0.3 * noise;

    return vec3(0.4 + lerp, 0.3 + lerp, 0.1 + lerp);
  "#, contents)
}
