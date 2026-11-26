class Base { value(): number { return 1; } }
class Derived extends Base { value(): number { return 2; } }
const b: Base = new Derived();
console.log(b.value());
