source "scattering.m"

rows = 1;
cols = 30;

cos_angle = linspace(-1, 1, cols);
g = linspace(0.75, 0.75, rows)';

figure 1;
plot(cos_angle, phase(repmat(cos_angle, rows, 1), repmat(g, 1, cols)));
title("phase");
