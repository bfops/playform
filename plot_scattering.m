source "scattering.m"

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

# we'll make one chart for each color, and then one beyond either extreme.

k = red_k;
figure 2;
plot(look_angle_v, in_scatter(sun_angle, camera_x, camera_y, look_x, look_y, k, rayleigh));
title("red");
legend("sunset");

k = green_k;
figure 3;
plot(look_angle_v, in_scatter(sun_angle, camera_x, camera_y, look_x, look_y, k, rayleigh));
title("green");
legend("sunset");

k = blue_k;
figure 4;
plot(look_angle_v, in_scatter(sun_angle, camera_x, camera_y, look_x, look_y, k, rayleigh));
title("blue");
legend("sunset");

k = red_k / (green/red) ^ -4;
figure 1;
plot(look_angle_v, in_scatter(sun_angle, camera_x, camera_y, look_x, look_y, k, rayleigh));
title("below red");
legend("sunset");

k = blue_k * ((blue/green)^-4);
figure 5;
plot(look_angle_v, in_scatter(sun_angle, camera_x, camera_y, look_x, look_y, k, rayleigh));
title("past blue");
legend("sunset");
