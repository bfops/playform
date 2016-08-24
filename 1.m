global planet_scale = 1000;
global planet_radius = 6400;

global planet_center_x = 0;
global planet_center_y = -planet_radius;

global atmos_thickness_ratio = 0.025;
global atmos_thickness = planet_radius * atmos_thickness_ratio;
global atmos_radius = planet_radius + atmos_thickness;

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

function [t1, t2] = intersect_time (o_x, o_y, r_x, r_y)
  a = dot(r_x, r_y, r_x, r_y);
  d_x = o_x - c_x;
  d_y = o_y - c_y;
  b = 2 .* dot(d_x, d_y, r_x, r_y);
  c = dot(d_x, d_y, d_x, d_y) - atmos_radius.*atmos_radius;

  s = b.*b - 4*a.*c;
  i = s < 0;

  t1 = ( sqrt(s) - b) ./ (2 .* a);
  t2 = (-sqrt(s) - b) ./ (2 .* a);
endfunction

function r = optical_depth(a_x, a_y, b_x, b_y)
  global atmos_radius;
  d_x = b_x - a_x;
  d_y = b_y - a_y;
  l = vec_len(d_x, d_y);
  samples = 100;
  d_x = d_x ./ l;
  d_y = d_y ./ l;
  l = min(l, 2 * atmos_radius) / samples;
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

  samples = 100;
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
camera_y = planet_radius;

max_k = 1/1;
k_samples = 1000;
k = max_k*[1:k_samples]/k_samples;

noon_up = in_scatter(3.14/2, camera_x, camera_y, 0, 1, k, 0);
noon_horizon = in_scatter(3.14/2, camera_x, camera_y, 1, 0, k, 0);
sunset_up = in_scatter(0, camera_x, camera_y, 0, 1, k, 0);
sunset_horizon = in_scatter(0, camera_x, camera_y, 1, 0, k, 0);

figure 1;
plot(k, [noon_up; noon_horizon; sunset_up; sunset_horizon]);

legend(["noon up"; "noon horizon"; "sunset up"; "sunset horizon"]);

rows = 40;
cols = 80;

camera_x = 0;
camera_y = ([0:(rows-1)]'/(rows-1))*2*atmos_thickness + planet_radius;
camera_y_mat = repmat(camera_y, 1, cols);

figure 2;
theta = (2*[0:cols-1]/(cols-1)-1) * 3.14/2;
theta_mat = repmat(theta, rows, 1);
look_x = cos(theta_mat);
look_y = sin(theta_mat);
sun_angle = 3.14/2;
y = in_scatter(sun_angle, camera_x, camera_y_mat, look_x, look_y, 1, 0);
y = y ./ repmat(in_scatter(sun_angle, camera_x, camera_y, 0, 1, 1, 0), 1, cols);
plot(theta, y);
title("in scattering vs view angle, sun at zenith, normalized");

figure 4;
theta = (2*[0:cols-1]/(cols-1)-1) * 3.14/2;
theta_mat = repmat(theta, rows, 1);
look_x = cos(theta_mat);
look_y = sin(theta_mat);
sun_angle = 3.14/4;
y = in_scatter(sun_angle, camera_x, camera_y_mat, look_x, look_y, 1, 0);
y = y ./ repmat(in_scatter(sun_angle, camera_x, camera_y, 0, 1, 1, 0), 1, cols);
plot(theta, y);
title("in scattering vs view angle, sun at 45, normalized");

figure 3;
theta = (2*[0:cols-1]/(cols-1)-1) * 3.14/2;
theta_mat = repmat(theta, rows, 1);
look_x = cos(theta_mat);
look_y = sin(theta_mat);
sun_angle = 0;
y = in_scatter(sun_angle, camera_x, camera_y_mat, look_x, look_y, 1, 0);
y = y ./ repmat(in_scatter(sun_angle, camera_x, camera_y, 0, 1, 1, 0), 1, cols);
plot(theta, y);
title("in scattering vs view angle, sun at horizon, normalized");
