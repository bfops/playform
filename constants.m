global sun_distance = 150000000;

global planet_scale = 1000;
global planet_radius = 6400;

global planet_center_x = 0;
global planet_center_y = -planet_radius;

global atmos_thickness_ratio = 0.025;
global atmos_thickness = planet_radius * atmos_thickness_ratio;
global atmos_radius = planet_radius + atmos_thickness;

global scale_height = atmos_thickness / 4;
