vec3 bark(vec3 world_position) {
  float total_amp = 0.0;
  float noise = 0.0;

  float freq[] = float[]( 64.0/1000, 128.0/1000, 256.0/1000, 1024.0/1000, 2048.0/1000 );
  float  amp[] = float[]( 1.0      , 0.6       , 0.4       , 0.4        , 0.4 );
  for (int i = 0; i < freq.length(); ++i) {
    float c = cnoise(freq[i] * world_position);
    noise += amp[i] * sign(c) * pow(abs(c), 0.2);
    total_amp += amp[i];
  }

  noise /= total_amp;
  noise = (noise + 1) / 2;

  return mix(vec3(0.4, 0.3, 0.1), vec3(0.7, 0.6, 0.4), noise);
}
