source "scattering.m"

rows = 40;
cols = 30;

camera_x = planet_center_x;
camera_y = planet_center_y + planet_radius;

look_angle_v = linspace(0, pi, cols);
look_angle = repmat(look_angle_v, rows, 1);
look_x = cos(look_angle);
look_y = sin(look_angle);

sun_angle = pi/8;

k = 2.^-[0:rows-1]';
k = repmat(k, 1, cols);

figure 1;
y = in_scatter(sun_angle, camera_x, camera_y, look_x, look_y, k, -0.75);
min = min(y')';
max = max(y')';
plot(look_angle_v, (y-min)./(max-min));
title("mie");
