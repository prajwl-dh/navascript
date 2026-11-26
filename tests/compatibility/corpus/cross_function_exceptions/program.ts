function fail(): void { throw 1; }
try { fail(); } catch (e) { console.log(e); }
