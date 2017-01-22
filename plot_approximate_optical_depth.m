source "approximate_optical_depth.m"

cols = 80;

camera_x = planet_center_x;
camera_y_v = planet_center_y + linspace(planet_radius, atmos_radius, cols);
camera_y = repmat(camera_y_v, 1, 1);

look_angle_v = pi;
look_angle = repmat(look_angle_v, 1, cols);

look_x = sin(look_angle);
look_y = -cos(look_angle);

l = 2 .* atmos_radius;
figure 1;
y = optical_depth(camera_x, camera_y, camera_x + look_x .* l, camera_y + look_y .* l);
approx = approximate_optical_depth(camera_x, camera_y, camera_x + look_x .* l, camera_y + look_y .* l);
plot(camera_y_v, [y; approx]);
title("Optical depth approximation");
legend("Optical depth", "Approximated optical depth");
