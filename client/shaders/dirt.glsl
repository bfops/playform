vec3 dirt(vec3 world_position) {
  float total_amp = 0.0;
  float noise = 0.0;

  for (int i = 0; i < sizeof(freq)/sizeof(freq[0]); ++i) {
    const float freq = 1 << i;
    const float amp = 1.0 / (1 << i);
    const float c = cnoise(freq * world_position);
    noise += amp * abs(c);
    total_amp += amp;
  }

  noise /= total_amp;
  noise = (noise + 1) / 2;

  return mix(vec3(0.4, 0.3, 0.1), vec3(0.7, 0.6, 0.4), noise);
}
