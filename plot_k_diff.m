source "scattering.m"

rows = 10;
cols = 30;

camera_x = planet_center_x;
camera_y = planet_center_y + planet_radius;

look_angle_v = linspace(0, pi, cols);
look_angle = repmat(look_angle_v, rows, 1);
look_x = cos(look_angle);
look_y = sin(look_angle);

k = 2 .^ -[0:rows-1]';
k = repmat(k, 1, cols);

# we'll make one chart for each color, and then one beyond either extreme.

figure 1;
sunset = in_scatter(0, camera_x, camera_y, look_x, look_y, k, rayleigh);
noon = in_scatter(pi/2, camera_x, camera_y, look_x, look_y, k, rayleigh);
y = sunset-noon;
max = repmat(max(y')', 1, cols);
min = repmat(min(y')', 1, cols);
plot(look_angle_v, (y-min)./(max-min));
legend("k=1");
