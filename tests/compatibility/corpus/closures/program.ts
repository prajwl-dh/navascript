function makeCounter(): () => number {
    let count: number = 0;
    return () => { count = count + 1; return count; };
}
const next: () => number = makeCounter();
console.log(next());
console.log(next());
