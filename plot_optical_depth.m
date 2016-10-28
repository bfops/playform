source "optical_depth.m"

rows = 40;
cols = 80;

camera_x = planet_center_x;
camera_y_v = planet_center_y + planet_radius + atmos_thickness*linspace(0, 1, cols);
camera_y = repmat(camera_y_v, rows, 1);

look_angle_v = 3.14/2 .* linspace(0, 1, rows)';
look_angle = repmat(look_angle_v, 1, cols);

look_x = cos(look_angle);
look_y = sin(look_angle);

y = optical_depth(camera_x, camera_y, camera_x + look_x .* atmos_radius, camera_y + look_y .* atmos_radius);
figure 1;
title("optical depth vs camera height at various look angles");
plot(camera_y_v, y);

mins = y(:, 1);
figure 2;
title("optical depth vs camera height at various look angles, normalized");
plot(camera_y_v, y./mins);

selected = (y./mins)(1,:);

figure 3;
title("optical depth vs camera height at various look angles, normalized, selected");
plot(camera_y_v, [selected; atmos_density(camera_x, camera_y_v)]);

p = polyfit(look_angle_v, mins, 7);
estimate = polyval(p, look_angle_v);
figure 4;
title("normalization factors vs look angle");
plot(look_angle_v, [mins estimate]);
