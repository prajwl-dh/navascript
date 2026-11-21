/* macOS entry point: bridges into Nava's synthetic `main` function. */
#include "nava_runtime.h"

extern void nava_main(void);

int main(void) {
    nava_main();
    return 0;
}
