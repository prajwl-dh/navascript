// Classes lower to named functions with `this` as the first parameter.
// Inheritance shares field layout and inherits methods; dispatch is static.
class Point {
    x: number;
    y: number;
    constructor(x: number, y: number) {
        this.x = x;
        this.y = y;
    }
    sum(): number {
        return this.x + this.y;
    }
}

class LabeledPoint extends Point {
    label: string;
    constructor(x: number, y: number, label: string) {
        super(x, y);
        this.label = label;
    }
    describe(): string {
        return this.label;
    }
}

// A derived field that sorts before a base field must not shift base slots.
class Shifted extends Point {
    a: number;
    constructor(x: number, y: number, a: number) {
        super(x, y);
        this.a = a;
    }
}

// A class without its own constructor inherits the base constructor.
class RenamedPoint extends Point {}

let p = new Point(2, 3);
console.log(p.sum());
let q = new LabeledPoint(4, 5, "q");
console.log(q.sum());
console.log(q.describe());
let r = new RenamedPoint(1, 2);
console.log(r.sum());
let s = new Shifted(10, 20, 1);
console.log(s.sum());
