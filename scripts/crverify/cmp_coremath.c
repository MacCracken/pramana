/* cmp_coremath.c — pramana's erf and erfc rows (scripts/crverify/rows_erf.cyr) against CORE-MATH's own build
 * (hardware FMA), bit for bit (M2 part b1; probe P9's cmp2.c, for `cr:`-prefixed rows). Linked with CORE-MATH's erf.c
 * and erfc.c built as orig_erf and orig_erfc by run.sh. Exit 1 on any difference. */
#include <stdio.h>
#include <string.h>
#include <stdint.h>
double orig_erf(double), orig_erfc(double);
static double fh(const char *s){ uint64_t u; sscanf(s, "%lx", &u); double d; memcpy(&d, &u, 8); return d; }
int main(void){
  char nm[64], xs[32], rs[32]; long n[2] = {0, 0}, bad[2] = {0, 0};
  while (scanf("%63s %31s %31s", nm, xs, rs) == 3) {
    const char *f0 = strncmp(nm, "cr:", 3) ? nm : nm + 3;
    if (strstr(f0, "_acc")) continue;              /* the accurate path alone: no C entry; MPFR checks it */
    int f = !strncmp(f0, "erfc", 4);
    double x = fh(xs), r = fh(rs), c = f ? orig_erfc(x) : orig_erf(x);
    uint64_t a, b; memcpy(&a, &r, 8); memcpy(&b, &c, 8); n[f]++;
    if (a != b) { if (bad[f] < 10) printf("%s %a: port %a, CORE-MATH %a\n", nm, x, r, c); bad[f]++; }
  }
  printf("erf: %ld rows, %ld differ from CORE-MATH; erfc: %ld rows, %ld differ\n", n[0], bad[0], n[1], bad[1]);
  return (bad[0] || bad[1]) ? 1 : 0;
}
