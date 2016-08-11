vec3 stone(vec3 world_position) {
  float total_amp = 0.0;
  float noise = 0.0;

  float freq[] = float[]( 2048.0/1000, 8192.0/1000, 65536.0/1000 );
  float  amp[] = float[]( 0.4        , 0.6        , 1.0 );
  for (int i = 0; i < freq.length(); ++i) {
    float c = cnoise(freq[i] * world_position);
    noise += amp[i] * sign(c) * pow(abs(c), 0.2);
    total_amp += amp[i];
  }

  noise /= total_amp;
  noise = (noise + 1) / 2;

  return mix(vec3(0.2, 0.2, 0.2), vec3(0.4, 0.4, 0.4), noise);
}
