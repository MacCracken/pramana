/* cbench.c — probe P9: erf and erfc in C on p9_erf.bcyr's regions (port plan §5 step 9e, §3.7). ns per call:
 * CORE-MATH with hardware FMA, the same source with emu.h's exact FMA emulation (stage 1), and glibc.
 *   bash scripts/probes/p9_erf.sh builds and runs it, pinned. */
#include <stdio.h>
#include <stdint.h>
#include <math.h>
#include <time.h>
double orig_erf(double), orig_erfc(double), nf_erf(double), nf_erfc(double);
static uint64_t s = 0x2545F4914F6CDD1Dull;
static double xs[4096];

static double unit(void) {
  s ^= s >> 12;
  s ^= s << 25;
  s ^= s >> 27;
  return (double)((s * 0x2545F4914F6CDD1Dull) >> 11) * 0x1p-53;
}

static void fill(double lo, double hi) {
  for (int i = 0; i < 4096; i++) xs[i] = lo + (hi - lo) * unit();
}

static void row(const char *name, double (*f)(double)) {
  struct timespec t0, t1;
  volatile double sink;
  double sum = 0;
  clock_gettime(CLOCK_MONOTONIC, &t0);
  for (int k = 0; k < 250; k++)
    for (int i = 0; i < 4096; i++) sum += f(xs[i]);
  clock_gettime(CLOCK_MONOTONIC, &t1);
  sink = sum;
  (void)sink;
  double ns = (t1.tv_sec - t0.tv_sec) * 1e9 + (t1.tv_nsec - t0.tv_nsec);
  printf("  %-44s %7.1f ns\n", name, ns / 1024000.0);
}

static void region(const char *what, double lo, double hi, int c) {
  fill(lo, hi);
  printf("%s\n", what);
  row("CORE-MATH, hardware FMA", c ? orig_erfc : orig_erf);
  row("CORE-MATH, FMA emulated (stage 1)", c ? nf_erfc : nf_erf);
  row(c ? "glibc erfc" : "glibc erf", c ? erfc : erf);
}

int main(void) {
  region("erf,  x in [1e-4, 1/16)", 0.0001, 0.0625, 0);
  region("erf,  x in [1/16, 5.9)", 0.0625, 5.9, 0);
  region("erfc, x in (-5.8, 0)", -5.8, 0.0, 1);
  region("erfc, x in [0, 2.88]", 0.0, 2.88, 1);
  region("erfc, x in [2.9, 25.8] (asymptotic)", 2.9, 25.8, 1);
  return 0;
}
