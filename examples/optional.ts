interface Box { name: string; }
let box: Box = { name: "nava" };
console.log(box?.name);

let words: string[] = ["a", "b"];
console.log(words?.[1]);
