interface Named { name: string; nickname?: string; }
const p: Named = { name: "navascript" };
console.log(p.name);
console.log(p.nickname == null);
