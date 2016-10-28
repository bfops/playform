source "scattering.m"

rows = 40;
cols = 80;

camera_x = planet_center_x;
camera_y_v = planet_center_y + planet_radius + atmos_thickness*([0:cols-1] ./ (cols-1));
camera_y = repmat(camera_y_v, rows, 1);

look_angle_v = 3.14/2 .* [0:rows-1]' ./ (rows-1);
look_angle = repmat(look_angle_v, 1, cols);

look_x = cos(look_angle);
look_y = sin(look_angle);

y = optical_depth(camera_x, camera_y, camera_x + look_x .* atmos_radius, camera_y + look_y .* atmos_radius);
figure 1;
title("optical depth vs camera height at various look angles");
plot(camera_y_v, y);
