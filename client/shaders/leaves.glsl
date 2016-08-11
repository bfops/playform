vec3 leaves(vec3 world_position) {
  float total_amp = 0.0;
  float noise = 0.0;

  float freq[] = { 8.0, 16.0, 2.0 };
  float  amp[] = { 1.0,  0.5, 0.8 };
  for (int i = 0; i < sizeof(freq)/sizeof(freq[0]); ++i) {
    const float c = cnoise(freq[i] * world_position);
    noise += amp[i] * sign(dnoise) * pow(abs(dnoise), 0.2);
    total_amp += amp[i];
  }

  noise /= total_amp;
  noise = (noise + 1) / 2;

  return mix(vec3(0.1, 0.4, 0.0), vec3(0.3, 0.6, 0.0), noise);
}
