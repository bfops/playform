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

function r = optical_depth(a_x, a_y, b_x, b_y)
  global atmos_radius;
  global planet_center_x;
  global planet_center_y;

  % "r" for "ray"
  r_x = b_x - a_x;
  r_y = b_y - a_y;

  a = dot(r_x, r_y, r_x, r_y);
  d_x = a_x - planet_center_x;
  d_y = a_y - planet_center_y;
  b = 2 .* dot(d_x, d_y, r_x, r_y);
  c = dot(d_x, d_y, d_x, d_y) - atmos_radius.*atmos_radius;

  s = b.*b - 4*a.*c;
  sign = s < 0;

  t1 = ( sqrt(s) - b) ./ (2 .* a);
  t2 = (-sqrt(s) - b) ./ (2 .* a);

  l = vec_len(r_x, r_y);
  samples = 100;
  r_x = r_x ./ l;
  r_y = r_y ./ l;
  l = min(l, 2 * atmos_radius) / samples;
  r_x = r_x .* l;
  r_y = r_y .* l;

  r = 0;
  for i = [1:samples]
    p_x = a_x + i .* r_x;
    p_y = a_y + i .* r_y;
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

%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%

rows = 40;
cols = 80;

camera_x = 0;
camera_y_v = planet_radius .* (1 + [0:cols-1] ./ cols);
camera_y = repmat(camera_y_v, rows, 1);

look_angle = 3.14 ./ 2 .* [1:rows]' ./ rows;
look_angle = repmat(look_angle, 1, cols);

look_x = cos(look_angle);
look_y = sin(look_angle);

figure 1;
y = optical_depth(camera_x, camera_y, camera_x + 2.* look_x .* planet_radius, camera_y + 2 .* look_y .* planet_radius);
plot(camera_y_v', log(y));
title("optical depth vs camera height, looking up");

%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%

camera_x = 0;
camera_y = planet_radius;

rows = 40;
cols = 80;

look_angle = ([0:cols-1]/(cols-1)) * 3.14/2;
look_angle_mat = repmat(look_angle, rows, 1);
look_x = cos(look_angle_mat);
look_y = sin(look_angle_mat);

sun_angle = ([0:rows-1]'/(rows-1)) * 3.14/2;
sun_angle_mat = repmat(sun_angle, 1, cols);

figure 3;
y = in_scatter(sun_angle_mat, camera_x, camera_y, look_x, look_y, 1, 0);
plot(look_angle, y);
title("in scattering vs view angle");

figure 4;
mins = in_scatter(sun_angle_mat, camera_x, camera_y, 1, 0, 1, 0);
plot(sun_angle, mins');
title("in scattering vs sun angle, looking at horizon");

figure 5;
maxs = in_scatter(sun_angle_mat, camera_x, camera_y, 0, 1, 1, 0);
plot(sun_angle, maxs');
title("in scattering vs sun angle, looking straight up");

figure 6;
y = (y - mins) ./ (maxs - mins);
plot(look_angle, y);
title("in scattering vs view angle, normalized");
