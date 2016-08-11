vec3 grass(vec3 world_position) {
  float total_amp = 0.0;
  float noise = 0.0;

  float freq[] = { 32,  16, 4   };
  float amp[]  = { 1 , 0.8, 0.8 };
  for (int i = 0; i < sizeof(freq)/sizeof(freq[0]); ++i) {
    const float c = cnoise(freq[i] * world_position);
    noise += amp[i] * sign(c) * pow(abs(c), 0.2);
    total_amp += amp[i];
  }

  noise += cnoise(0.25 * world_position);
  total_amp += 1;

  noise /= total_amp;
  noise = (noise + 1) / 2;

  return mix(vec3(0.1, 0.4, 0.0), vec3(0.3, 0.6, 0.0), noise);
}
