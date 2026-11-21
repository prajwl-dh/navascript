let hex = 0xFF;
let binary = 0b1010;
let octal = 0o17;
let separated = 1_000;
console.log(hex);
console.log(binary);
console.log(octal);
console.log(separated);

let x = 10;
x += 5;
console.log(x);
x -= 3;
console.log(x);
x *= 2;
console.log(x);
x /= 4;
console.log(x);
x %= 3;
console.log(x);

let y = 2;
console.log(y ** 3);
console.log(++y);
console.log(y++);
console.log(y);
console.log(--y);
console.log(y--);
console.log(y);

let greeting = "hi";
greeting += "!";
console.log(greeting);

console.log("\x41\u{42}\u0043");

let café = 3;
console.log(café);

console.log(1 > 0 ? "yes" : "no");
