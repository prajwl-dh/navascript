let total: number = 0;
for (let i = 0; i < 5; i = i + 1) {
    if (i == 3) { continue; }
    if (i == 4) { break; }
    total += i;
}
console.log(total);
const n: number = 5;
console.log(n > 2 ? "big" : "small");
