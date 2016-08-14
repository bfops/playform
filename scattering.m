k = 1;
r_atmos = 4*k;
max_h = r_atmos * 2;

cols = 100;
rows = 21;
hs = max_h*[1:cols]/cols;
h = repmat(hs, rows, 1);
ys = (2.*[0:rows-1]'./(rows-1)-1)*3.14/2;
y = repmat(ys, 1, cols);
s = sin(y);
c = cos(y);

sun_x = 1000;
sun_y = h;

r1 = repmat(0, rows, cols);
step = 10;
max_t = 4*max_h;
for t = [1:max_t*step]/step
  point_x = t .* c;
  point_y = h + t .* s;

  l = sqrt(point_x.*point_x + point_y._point_y);

  step2 = 10
  for t2 = [1:step2]/step2
    point2_x = t2 .* c;
    point2_y = h + t2 .* s;
    l2 = sqrt(point2_x.*point2_x + point2_y.*point2_y);
    r1 += exp(-k .* (l + l2));
  end

  d_x = sun_x - point_x;
  d_y = sun_y - point_y;
  l = sqrt(d_x.*d_x + d_y.*d_y);
  d_x = d_x ./ l;
  d_y = d_y ./ l;
  for t2 = [1:max_t*step]/step
    point2_x = point_x + t2 .* d_x;
    point2_y = point_y + t2 .* d_y;
    l2 = sqrt(point_x.*point_x + point_y.*point_y);
    r1 += exp(-k .* l2);
  end
endfor

figure 1;
plot(hs, r1);
axis([0 max_h 0 2]);

figure 2;
%plot(ys, r1');
axis([-3.14/2 3.14/2 0 2]);
