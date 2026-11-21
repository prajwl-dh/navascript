/* Minimal native runtime for Nava programs (Phase 4).
 * The full runtime, including strings and collections, arrives in Phase 5. */
#include <ctype.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* macOS entry point bridges into Nava's synthetic `main` function. */
extern void nava_main(void);

int main(void) {
    nava_main();
    return 0;
}

void nava_console_log_bool(int value) {
    printf("%s\n", value ? "true" : "false");
}

/* Print a number the way TypeScript's Number.toString mostly does: integer
 * notation for whole values, fixed notation in the normal range, and exponent
 * notation only for very small or very large magnitudes. */
void nava_console_log_f64(double value) {
    char general[64];
    char buffer[64];

    if (value == (double)(long long)value && value >= -9e18 && value <= 9e18) {
        printf("%lld\n", (long long)value);
        return;
    }

    snprintf(general, sizeof(general), "%.15g", value);
    char *exponent = strchr(general, 'e');
    if (exponent == NULL) {
        printf("%s\n", general);
        return;
    }

    int power = atoi(exponent + 1);
    double magnitude = fabs(value);
    if (magnitude < 1e-6 || magnitude >= 1e21) {
        printf("%s\n", general);
        return;
    }

    /* Fixed notation with just enough decimals to preserve the digits that
     * %.15g considered significant, then trim trailing zeroes. */
    int digits = 0;
    for (char *cursor = general; cursor < exponent; ++cursor) {
        if (isdigit((unsigned char)*cursor)) {
            ++digits;
        }
    }
    int precision = digits - (power + 1);
    if (precision < 0) {
        precision = 0;
    }
    if (precision > 17) {
        precision = 17;
    }
    snprintf(buffer, sizeof(buffer), "%.*f", precision, value);
    size_t length = strlen(buffer);
    while (length > 0 && buffer[length - 1] == '0') {
        buffer[--length] = '\0';
    }
    if (length > 0 && buffer[length - 1] == '.') {
        buffer[--length] = '\0';
    }
    printf("%s\n", buffer);
}
