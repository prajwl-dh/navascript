// Generic functions are monomorphized at each call site. Type arguments can be
// inferred from the arguments or written explicitly.
function identity<T>(value: T): T {
    return value;
}

function first<T>(items: T[]): T {
    return items[0];
}

interface Box<T> {
    value: T;
}

console.log(identity(42));
console.log(identity("nava"));
console.log(identity<boolean>(true));
console.log(first([1, 2, 3]));
console.log(first(["a", "b"]));

let box: Box<number> = { value: 7 };
console.log(box.value);
