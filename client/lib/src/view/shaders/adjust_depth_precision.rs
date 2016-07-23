pub fn as_string(near: f32, far: f32) -> String {
  format!(
    r#"
      // http://outerra.blogspot.ca/2012/11/maximizing-depth-buffer-range-and.html
      vec4 adjust_depth_precision(vec4 p) {{
        float near = {};
        float far = {};
        p.z = 2.0*log(p.w/near)/log(far/near) - 1;
        p.z *= p.w;
        return p;
      }}
    "#,
    near,
    far
  )
}
