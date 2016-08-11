// http://outerra.blogspot.ca/2012/11/maximizing-depth-buffer-range-and.html
vec4 adjust_depth_precision(float near, float far, vec4 p) {{
  p.z = 2.0*log(p.w/near)/log(far/near) - 1;
  p.z *= p.w;
  return p;
}}
