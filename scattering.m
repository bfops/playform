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

function a_x, a_y, r_x, r_y = clip_ray(a_x, a_y, r_x, r_y)
  global atmos_radius;
  global planet_center_x;
  global planet_center_y;

  % find the times where the ray intersects the atmosphere
  a = dot(r_x, r_y, r_x, r_y);
  d_x = a_x - planet_center_x;
  d_y = a_y - planet_center_y;
  b = 2 .* dot(d_x, d_y, r_x, r_y);
  c = dot(d_x, d_y, d_x, d_y) - atmos_radius.*atmos_radius;
  s = b.*b - 4*a.*c;
  t1 = (-sqrt(s) - b) ./ (2 .* a);
  t2 = ( sqrt(s) - b) ./ (2 .* a);

  % if s < 0, we don't intersect. Set both times to 0.
  sign = s >= 0;
  t1 = t1 .* sign;
  t2 = t2 .* sign;

  % negative times mean either we're in the atmosphere, or it's entirely behind us.
  t1 = max(t1, 0);
  t2 = max(t2, 0);

  % reset [a] and [r] to be inside the atmosphere.
  a_x = a_x + r_x .* t1;
  a_y = a_y + r_y .* t1;
  r_x = r_x .* (t2 - t1);
  r_y = r_y .* (t2 - t1);
endfunction

function r = optical_depth(a_x, a_y, b_x, b_y)
  global atmos_radius;

  % "r" for "ray"
  r_x = b_x - a_x;
  r_y = b_y - a_y;

  %a_x, a_y, r_x, r_y = clip_ray(a_x, a_y, b_x, b_y);

  samples = 100;
  l = vec_len(r_x, r_y) ./ samples;
  r_x = r_x ./ samples;
  r_y = r_y ./ samples;

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
camera_y_v = planet_radius + atmos_radius.*([0:cols-1] ./ (cols-1));
camera_y = repmat(camera_y_v, rows, 1);

look_angle_v = 3.14/2 .* [0:rows-1]' ./ (rows-1);
look_angle = repmat(look_angle_v, 1, cols);

look_x = cos(look_angle);
look_y = sin(look_angle);

y = optical_depth(camera_x, camera_y, camera_x + look_x .* atmos_radius, camera_y + look_y .* atmos_radius);
figure 1;
plot(camera_y_v', log(y));
title("log(optical depth) vs camera height, various look angles");

mins = optical_depth(camera_x, camera_y, camera_x, camera_y + atmos_radius);
figure 2;
plot(look_angle_v, (y./mins));
title("optical depth vs look angle at various heights, normalized");

selected = optical_depth(camera_x, planet_radius, camera_x + cos(look_angle_v) .* atmos_radius, planet_radius + sin(look_angle_v) .* atmos_radius);

figure 3;
scatter(look_angle_v, selected);
title("optical depth vs look angle at various heights, selected");

figure 4;
plot(camera_y_v, log(mins));
title("log(normalization factors) vs camera height");

%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%

%camera_x = 0;
%camera_y = planet_radius;
%
%rows = 40;
%cols = 200;
%
%camera_y_v = planet_radius + atmos_radius.*([0:cols-1] ./ cols);
%camera_y = repmat(camera_y_v, rows, 1);
%
%look_angle_v = ([1:rows]'./rows) * 3.14;
%look_angle = repmat(look_angle_v, 1, cols);
%look_x = cos(look_angle);
%look_y = sin(look_angle);
%
%figure 3;
%y = in_scatter(0, camera_x, camera_y, look_x, look_y, 1, 0);
%plot(look_angle_v, y);
%title("in scattering vs look angle, sun at horizon, various heights");
%
%figure 4;
%y = in_scatter(3.14/2, camera_x, camera_y, look_x, look_y, 1, 0);
%plot(look_angle_v, y);
%title("in scattering vs look angle, sun at zenith, various heights");
%
%figure 5;
%y = in_scatter(0, camera_x, camera_y, look_x, look_y, 1, 0);
%y = y ./ in_scatter(0, camera_x, camera_y, 0, 1, 1, 0);
%plot(look_angle_v, y);
%title("in scattering vs look angle, sun at horizon, various heights, normalized");
%
%figure 6;
%y = in_scatter(3.14/2, camera_x, camera_y, look_x, look_y, 1, 0);
%y = y ./ in_scatter(3.14/2, camera_x, camera_y, 0, 1, 1, 0);
%plot(look_angle_v, y);
%title("in scattering vs look angle, sun at zenith, various heights, normalized");
%
%figure 7;
%y = in_scatter(0, camera_x, camera_y, 0, 1, 1, 0);
%plot(camera_y_v, log(y));
%title("log(in scattering) vs height, looking straight up, sun at horizon");
%
%figure 8;
%y = in_scatter(3.14/2, camera_x, camera_y, 0, 1, 1, 0);
%plot(camera_y_v, log(y));
%title("log(in scattering) vs height, looking straight up, sun at zenith");
