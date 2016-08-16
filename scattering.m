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
  global atmos_radius;

  sun_x = cos(sun_angle);
  sun_y = sin(sun_angle);
  sun_depth = atmos_radius .* sun_x + atmos_thickness .* sun_y;
  look_depth = atmos_radius .* look_x + atmos_thickness .* look_y;
  cos_theta = dot(sun_x, sun_y, look_x, look_y);
  r = 0;
  r += k .* (sun_depth + 1) .* (sun_depth.*sun_depth + look_depth.*look_depth - 2*sun_depth.*look_depth);
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

rows = 20;
cols = 40;

camera_x = 0;
camera_y = ([0:(rows-1)]'/(rows-1))*2*atmos_thickness + planet_radius;
camera_y_mat = repmat(camera_y, 1, cols);

%figure 2;
%theta = ([0:cols-1]/(cols-1)-1) * 3.14/2;
%theta_mat = repmat(theta, rows, 1);
%look_x = cos(theta_mat);
%look_y = sin(theta_mat);
%sun_angle = 3.14/2;
%y = in_scatter(sun_angle, camera_x, camera_y_mat, look_x, look_y, 1, 0);
%y = y ./ repmat(y(:,1), 1, cols);
%plot(theta, y);
%title("atmos density to sun vs view angle, sun at zenith");
%
%figure 3;
%theta = ([0:cols-1]/(cols-1)-1) * 3.14/2;
%theta_mat = repmat(theta, rows, 1);
%look_x = cos(theta_mat);
%look_y = sin(theta_mat);
%sun_angle = 0;
%y = in_scatter(sun_angle, camera_x, camera_y_mat, look_x, look_y, 1, 0);
%y = y ./ repmat(y(:,1), 1, cols);
%plot(theta, y);
%title("atmos density to sun vs view angle, sun at horizon");
%
%figure 4;
%theta = 2 * (2*[0:cols-1]/(cols-1)-1) * 3.14/2;
%theta_mat = repmat(theta, rows, 1);
%look_x = cos(theta_mat);
%look_y = sin(theta_mat);
%sun_angle = 3.14/2;
%y = in_scatter(sun_angle, camera_x, camera_y_mat, look_x, look_y, 1, 0);
%% normalize each line by their theta=0 values (i.e. when the camera is pointed straight up).
%y = y ./ repmat(in_scatter(sun_angle, camera_x, camera_y, 0, 1, 1, 0), 1, cols);
%y_approx = 1.5 .* (1 - sin(theta));
%plot(theta, y);
%title("atmospheric density vs view angle, normalized");
%
%figure 5;
%sun_angle = 3.14/2;
%y = log(in_scatter(sun_angle, camera_x, camera_y, 0, 1, 1, 0));
%y_approx = planet_radius ./ planet_scale - camera_y ./ scale_height;
%% graph the theta=0 values vs initial height.
%plot(camera_y', [y'; y_approx']);
%title("log(atmos density to sun) vs height, sun at zenith");
