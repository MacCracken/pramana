/* Probe P9 stage 2: the Cyrius port's rows against CORE-MATH's own build (hardware FMA), bit for bit. */
#include <stdio.h>
#include <string.h>
#include <stdint.h>
double orig_erf(double), orig_erfc(double);
static double fh(const char *s){ uint64_t u; sscanf(s, "%lx", &u); double d; memcpy(&d, &u, 8); return d; }
int main(void){
  char nm[64], xs[32], rs[32]; long n[2] = {0, 0}, bad[2] = {0, 0};
  while (scanf("%63s %31s %31s", nm, xs, rs) == 3) {
    if (strstr(nm, "_acc")) continue;              /* the accurate path alone: no C entry; MPFR checks it */
    int f = !strncmp(nm, "erfc", 4);
    double x = fh(xs), r = fh(rs), c = f ? orig_erfc(x) : orig_erf(x);
    uint64_t a, b; memcpy(&a, &r, 8); memcpy(&b, &c, 8); n[f]++;
    if (a != b) { if (bad[f] < 10) printf("%s %a: port %a, CORE-MATH %a\n", nm, x, r, c); bad[f]++; }
  }
  printf("erf: %ld rows, %ld differ from CORE-MATH; erfc: %ld rows, %ld differ\n", n[0], bad[0], n[1], bad[1]);
  return 0; }
