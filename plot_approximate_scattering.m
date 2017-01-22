source "approximate_scattering.m"

rows = 10;
cols = 30;

camera_x = planet_center_x;
camera_y = planet_center_y + planet_radius;

look_angle_v = linspace(0, pi, cols);
look_angle = repmat(look_angle_v, rows, 1);
look_x = cos(look_angle);
look_y = sin(look_angle);

sun_angle = linspace(0, pi/2, rows)';
sun_angle = repmat(sun_angle, 1, cols);

k = red_k;
figure 6;
plot(look_angle_v, approximate_in_scatter(sun_angle, camera_x, camera_y, look_x, look_y, k, rayleigh));
title("approximate red");
legend("sunset");

k = green_k;
figure 7;
plot(look_angle_v, approximate_in_scatter(sun_angle, camera_x, camera_y, look_x, look_y, k, rayleigh));
title("approximate green");
legend("sunset");

k = blue_k;
figure 8;
plot(look_angle_v, approximate_in_scatter(sun_angle, camera_x, camera_y, look_x, look_y, k, rayleigh));
title("approximate blue");
legend("sunset");

k = red_k / (green/red) ^ -4;
figure 9;
plot(look_angle_v, approximate_in_scatter(sun_angle, camera_x, camera_y, look_x, look_y, k, rayleigh));
title("approximate below red");
legend("sunset");

k = blue_k * ((blue/green)^-4);
figure 10;
plot(look_angle_v, approximate_in_scatter(sun_angle, camera_x, camera_y, look_x, look_y, k, rayleigh));
title("approximate past blue");
legend("sunset");
