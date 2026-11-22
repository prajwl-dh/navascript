// Optional call syntax: parsed and type-checked like a direct call. The null
// branch is unobservable until Nava has nullable callable values.
function greet(name: string): string {
    return "hi " + name;
}
console.log(greet?.("nava"));
