interface Point { x: number; y: number; }
const values: number[] = [1, 2, 3];
console.log(values.length);
values[1] = 42;
console.log(values[1]);
const p: Point = { x: 3, y: 4 };
console.log(p.x + p.y);
p.y = 9;
console.log(p.y);
