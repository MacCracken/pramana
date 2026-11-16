/* fmatest.c — probe P9 stage 1: emu.h's emu_fma against hardware fma (built with -mfma) on 10^8 seeded triples,
   weighted to cancellation and to an addend far below or above the product. */
#include <stdio.h>
#include <math.h>
#include "emu.h"
static uint64_t s = 0x9E3779B97F4A7C15ull;
static uint64_t nx(void){ s ^= s >> 12; s ^= s << 25; s ^= s >> 27; return s * 0x2545F4914F6CDD1Dull; }
/* a double with a random sign, significand and an exponent in [lo, hi] */
static double rd(int lo, int hi){ uint64_t r = nx(); int ex = lo + (int)(nx() % (uint64_t)(hi - lo + 1));
  return ldexp(1.0 + (double)(r >> 12) * 0x1p-52, ex) * ((r & 1) ? -1.0 : 1.0); }
int main(void){
  long bad = 0, n = 0;
  for (long i = 0; i < 100000000; i++) {
    double a = rd(-300, 300), b = rd(-300, 300);
    int pe = ilogb(a) + ilogb(b);
    double c;
    switch (i % 4) { /* c near a*b (cancellation), far below, far above, or anywhere nearby */
      case 0: c = -a * b * (1.0 + (double)(nx() >> 40) * 0x1p-60); break;
      case 1: c = rd(pe - 120, pe - 40); break;
      case 2: c = rd(pe + 10, pe + 60); break;
      default: c = rd(pe - 60, pe + 5); }
    double h = fma(a, b, c), m = emu_fma(a, b, c); n++;
    if (emu_bits(h) != emu_bits(m)) { if (bad < 5) printf("a=%a b=%a c=%a fma=%a emu=%a\n", a, b, c, h, m); bad++; }
  }
  printf("%ld triples, %ld differ\n", n, bad);
  return 0; }
