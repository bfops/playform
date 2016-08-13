k = 1;
max_x = 100;

cols = 100;
rows = 41;
xs = max_x*[1:cols]/cols;
x = repmat(xs, rows, 1);
ys = (2.*[0:rows-1]'./(rows-1)-1)*3.14/2;
y = repmat(ys, 1, cols);
s = sin(y);

r1 = repmat(0, rows, cols);
step = 10;
max_t = 1000;
for t = [1:max_t*step]/step
  r1 += exp(-k*sqrt(x.*x + 2.*s.*x.*t + t.*t)) * 1/step;
endfor

figure 1;
plot(xs, r1);
axis([0 max_x 0 2]);

figure 2;
plot(ys, r1');
axis([-3.14/2 3.14/2 0 2]);
