/* cmp.c — probe P9 stage 1 (port plan §5 step 9e): the FMA-free C build of CORE-MATH's erf and erfc (every
 * __builtin_fma replaced by emu.h's exact emulation) against CORE-MATH's own build with hardware FMA, bit for bit,
 * on both worst-case lists (each input and its negation) and 20 million seeded arguments per function.
 *   ./cmp build/p9/coremath/erf.wc build/p9/coremath/erfc.wc
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <math.h>
double orig_erf(double), nf_erf(double), orig_erfc(double), nf_erfc(double);
static uint64_t bits(double x) { uint64_t u; memcpy(&u, &x, 8); return u; }
static uint64_t s = 0x2545F4914F6CDD1Dull;
static uint64_t nx(void) { s ^= s >> 12; s ^= s << 25; s ^= s >> 27; return s * 0x2545F4914F6CDD1Dull; }
static long n, bad;

static void one(int f, double x) {
  double a = f ? orig_erfc(x) : orig_erf(x), b = f ? nf_erfc(x) : nf_erf(x);
  n++;
  if (bits(a) != bits(b)) {
    if (bad < 10) printf("%s(%a): coremath %a, fma-free %a\n", f ? "erfc" : "erf", x, a, b);
    bad++;
  }
}

/* A worst-case line: a hex float, or one of ±snan, ±nan, ±inf, ±0 (strtod reads all but snan). */
static double parse(const char *line) {
  if (strstr(line, "snan")) return line[0] == '-' ? -NAN : NAN;
  return strtod(line, 0);
}

int main(int argc, char **argv) {
  if (argc != 3) { fprintf(stderr, "usage: cmp erf.wc erfc.wc\n"); return 2; }
  for (int f = 0; f < 2; f++) {
    n = bad = 0;
    FILE *in = fopen(argv[1 + f], "r");
    if (!in) { perror(argv[1 + f]); return 2; }
    char line[256];
    while (fgets(line, sizeof line, in)) {
      if (line[0] == '#' || line[0] == '\n') continue;
      double x = parse(line);
      one(f, x);
      one(f, -x);
    }
    fclose(in);
    long wn = n, wb = bad;
    for (long i = 0; i < 20000000; i++) {
      uint64_t r = nx();
      double x;
      if (i % 4 == 0) x = (double)(r >> 11) * 0x1p-53 * (f ? 34.0 : 12.0) - 6.0;                 /* the main range */
      else if (i % 4 == 1) x = ldexp(1.0 + (double)(r >> 12) * 0x1p-52, -(int)(nx() % 1080)) * ((r & 1) ? -1 : 1);
      else if (i % 4 == 2) { uint64_t u = nx(); memcpy(&x, &u, 8); if (isnan(x)) x = 1.0; }      /* any bits */
      else x = f ? 25.0 + (double)(r >> 11) * 0x1p-53 * 2.5 : 5.8 + (double)(r >> 11) * 0x1p-53 * 0.2;  /* tails */
      one(f, x);
    }
    printf("%s: worst cases %ld (%ld differ); all %ld, %ld differ\n", f ? "erfc" : "erf", wn, wb, n, bad);
  }
  return 0;
}
