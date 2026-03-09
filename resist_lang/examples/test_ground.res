// Quick test: RC filter with gnd to verify no singular matrix
let vsrc = VSource(input, gnd, 5).pos(80, 150).rot(90);
let r1 = Resistor(input, output, 1k).pos(200, 80);
let c1 = Capacitor(output, gnd, 100n).pos(320, 150).rot(90);

wire(80, 80, 200, 80);
wire(200, 80, 320, 80);
wire(80, 220, 320, 220);

analyze.dc();
analyze.transient(stop: 1m, step: 1u);
