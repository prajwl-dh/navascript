/* macOS entry point: bridges OS arguments into NavaScript's synthetic entry. */
#include "navascript_runtime.h"

extern void navascript_main(void);

int main(int argc, char **argv) {
    navascript_gc_init();
    navascript_init_io(argc, argv);
    navascript_main();
    return 0;
}
