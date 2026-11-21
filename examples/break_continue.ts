function find(n: number): number {
    let i = 0;
    while (i < 10) {
        i = i + 1;
        if (i == 4) {
            break;
        }
    }
    return i;
}
console.log(find(0));

let total = 0;
for (let j = 0; j < 6; j = j + 1) {
    if (j % 2 == 0) {
        continue;
    }
    total = total + j;
}
console.log(total);

let steps = 0;
do {
    steps = steps + 1;
    if (steps < 3) {
        continue;
    }
    break;
} while (steps < 10);
console.log(steps);

let forever = 0;
for (;;) {
    forever = forever + 1;
    if (forever < 2) {
        continue;
    }
    break;
}
console.log(forever);

let found = 0;
for (let x of [1, 2, 3]) {
    if (x == 2) {
        continue;
    }
    found = found + x;
}
console.log(found);

let pairs = 0;
for (let i = 0; i < 2; i = i + 1) {
    for (let j = 0; j < 2; j = j + 1) {
        if (j == 1) {
            continue;
        }
        pairs = pairs + 1;
    }
}
console.log(pairs);

let k = 0;
while (k < 3) {
    k = k + 1;
    continue;
}
console.log(k);
