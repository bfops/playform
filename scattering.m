global planet_scale = 1000;
global planet_radius = 6400;

global planet_center_x = 0;
global planet_center_y = -planet_radius;

global atmos_thickness_ratio = 0.025;
global atmos_thickness = planet_radius * atmos_thickness_ratio;

global scale_height = atmos_thickness / 4;

function r = vec_len(d_x, d_y)
  r = sqrt(d_x .* d_x + d_y .* d_y);
endfunction

function r = atmos_density (p_x, p_y)
  global planet_center_x;
  global planet_center_y;
  global planet_radius;
  global scale_height;
  d_x = p_x - planet_center_x;
  d_y = p_y - planet_center_y;
  dist_from_surface = vec_len(d_x, d_y) - planet_radius;
  r = exp(-dist_from_surface / scale_height);
endfunction

function r = optical_depth(a_x, a_y, b_x, b_y)
  global atmos_thickness;
  global planet_radius;
  samples = 5;
  d_x = b_x - a_x;
  d_y = b_y - a_y;
  l = vec_len(d_x, d_y);
  d_x = d_x ./ l;
  d_y = d_y ./ l;
  l = min(l, 2 * (atmos_thickness + planet_radius)) / samples;
  d_x = d_x .* l;
  d_y = d_y .* l;

  r = 0;
  for i = [1:samples]
    p_x = a_x + i .* d_x;
    p_y = a_y + i .* d_y;
    r = r + atmos_density(p_x, p_y) .* l;
  endfor
endfunction

function r = phase(cos_angle, g)
  c = cos_angle;
  g2 = g .* g;
  r = 3 * (1-g2) .* (1 + c.*c) ./ (2 * (2 + g2) .* realpow(1 + g2 - 2*g.*c, 3.0 / 2.0));
endfunction

function r = dot(x1, y1, x2, y2)
  r = x1.*x2 + y1.*y2;
endfunction

function r = in_scatter(sun_angle, camera_x, camera_y, look_x, look_y, k, g)
  global planet_center_x;
  global planet_center_y;
  global planet_radius;
  global atmos_thickness;
  sun_distance = 150000000;
  sun_position_x = planet_center_x + cos(sun_angle) .* sun_distance;
  sun_position_y = planet_center_y + sin(sun_angle) .* sun_distance;

  samples = 5;
  l = atmos_thickness / samples;
  r = 0;
  for i = [1:samples]
    point_x = camera_x + look_x * i * l;
    point_y = camera_y + look_y * i * l;
    d1_x = sun_position_x - point_x;
    d1_y = sun_position_y - point_y;
    d2_x = point_x - camera_x;
    d2_y = point_y - camera_y;
    cos_angle = dot(d1_x, d1_y, d2_x, d2_y) ./ (vec_len(d1_x, d1_y) .* vec_len(d2_x, d2_y));
    od = optical_depth(camera_x, camera_y, point_x, point_y) + optical_depth(point_x, point_y, sun_position_x, sun_position_y);
    r += k .* phase(cos_angle, g) .* atmos_density(point_x, point_y) .* exp(-k .* od) * l;
  endfor
endfunction

camera_x = 0;
camera_y = 0;

max_k = 1;
k_samples = 1000;
k = [1:max_k*k_samples]/k_samples;

noon_up = in_scatter(3.14/2, camera_x, camera_y, 0, 1, k, 0);
noon_horizon = in_scatter(3.14/2, camera_x, camera_y, 1, 0, k, 0);
sunset_up = in_scatter(0, camera_x, camera_y, 0, 1, k, 0);
sunset_horizon = in_scatter(0, camera_x, camera_y, 1, 0, k, 0);

figure 1;
plot(k, [noon_up]);
axis([0 1 0 1]);

figure 2;
plot(k, [noon_horizon]);
axis([0 1 0 1]);

figure 3;
plot(k, [sunset_up]);
axis([0 1 0 1]);

figure 4;
plot(k, [sunset_horizon]);
axis([0 1 0 1]);
