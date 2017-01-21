source "optical_depth.m"

rows = 40;
cols = 80;

camera_x = planet_center_x;
camera_y_v = planet_center_y + linspace(planet_radius, atmos_radius, cols);
camera_y = repmat(camera_y_v, rows, 1);

look_angle_v = linspace(pi/2, pi, rows)';
look_angle = repmat(look_angle_v, 1, cols);

look_x = sin(look_angle);
look_y = -cos(look_angle);

l = 2 .* atmos_radius;
y = optical_depth(camera_x, camera_y, camera_x + look_x .* l, camera_y + look_y .* l);
figure 1;
plot(camera_y_v, y);
title("optical depth vs camera height at various look angles");

mins = y(:, 1);
normalized = y ./ repmat(mins, 1, cols);
figure 2;
plot(camera_y_v, normalized);
title("optical depth vs camera height at various look angles, normalized");

selected = normalized(size(normalized,1),:);

figure 3;
plot(camera_y_v, [selected; atmos_density(camera_x, camera_y_v)]);
title("optical depth vs camera height at various look angles, normalized, selected");

p = polyfit(look_angle_v, mins, 7);
disp(p);
estimate = polyval(p, look_angle_v);
figure 4;
plot(look_angle_v, [mins estimate]);
title("normalization factors vs look angle");
