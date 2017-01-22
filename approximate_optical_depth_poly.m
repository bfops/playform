source "optical_depth.m"

function p = approximate_optical_depth_poly()
  global planet_center_x;
  global planet_center_y;
  global planet_radius;
  global atmos_radius;

  rows = 40;
  cols = 80;

  camera_x = planet_center_x;
  camera_y = planet_center_y + linspace(planet_radius, atmos_radius, cols);

  look_angle_v = linspace(pi/2, pi, rows)';
  look_angle = repmat(look_angle_v, 1, cols);

  look_x = sin(look_angle);
  look_y = -cos(look_angle);

  l = 2 .* atmos_radius;
  y = optical_depth(camera_x, camera_y, camera_x + look_x .* l, camera_y + look_y .* l);
  mins = y(:, 1);
  normalized = y ./ repmat(mins, 1, cols);

  p = polyfit(look_angle_v, mins, 3);
end
