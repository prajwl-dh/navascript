function run(): void {
    try {
        throw 7;
    } catch (e) {
        console.log(e);
    } finally {
        console.log("done");
    }
}
run();
