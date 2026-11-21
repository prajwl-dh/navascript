function sumTo(n: number): number {
    let total = 0;
    let i = 1;
    while (i <= n) {
        total = total + i;
        i = i + 1;
    }
    return total;
}
console.log(sumTo(5));

let product = 1;
for (let j = 1; j <= 4; j = j + 1) {
    product = product * j;
}
console.log(product);

let countdown = 0;
do {
    countdown = countdown + 1;
} while (countdown < 3);
console.log(countdown);

function firstOf(value: number): number {
    while (true) {
        return value;
    }
}
console.log(firstOf(7));
