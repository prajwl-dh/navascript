function identity<T>(value: T): T { return value; }
interface Box<T> { value: T; }
console.log(identity<number>(7));
const b: Box<string> = { value: "ok" };
console.log(b.value);
