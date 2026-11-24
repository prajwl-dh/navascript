/* macOS entry point: bridges into NavaScript's synthetic `main` function. */
#include "navascript_runtime.h"

extern void navascript_main(void);

int main(void) {
    navascript_main();
    return 0;
}
