let xs: number[] = [10, 20, 30];
let total = 0;
for (let x of xs) {
    total = total + x;
}
console.log(total);

let word = "abc";
let chars = "";
for (let c of word) {
    chars = chars + c;
}
console.log(chars);

let first = 0;
for (first of xs) {
    console.log(first);
}
