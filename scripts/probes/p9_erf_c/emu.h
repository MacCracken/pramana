/* Probe P9 stage 1: an exact fma without FMA hardware. a*b = p + e exactly (Dekker's product with Veltkamp's
   split), then RN(p + e + c) exactly (Boldo and Melquiond's round-to-odd sum; kimiya's _km_rn3). */
#include <stdint.h>
#include <string.h>
static inline uint64_t emu_bits(double x){ uint64_t u; memcpy(&u,&x,8); return u; }
static inline double emu_dbl(uint64_t u){ double x; memcpy(&x,&u,8); return x; }
static inline void emu_two_sum(double a, double b, double *s, double *e){
  double ss = a + b; double bb = ss - a; *e = (a - (ss - bb)) + (b - bb); *s = ss; }
static inline void emu_two_prod(double a, double b, double *p, double *e){
  double pp = a * b; const double sp = 134217729.0;
  double ca = sp * a; double ah = ca - (ca - a); double al = a - ah;
  double cb = sp * b; double bh = cb - (cb - b); double bl = b - bh;
  *e = (al * bl) - (((pp - (ah * bh)) - (al * bh)) - (ah * bl)); *p = pp; }
static inline double emu_ro_sum(double a, double b){
  double s, e; emu_two_sum(a, b, &s, &e);
  if (e == 0.0) return s;
  uint64_t sb = emu_bits(s);
  if (sb & 1) return s;
  int up = e > 0.0, pos = s > 0.0;
  sb = (up == pos) ? sb + 1 : sb - 1;
  return emu_dbl(sb); }
static inline double emu_rn3(double a, double b, double c){
  double uh, ul, th, tl; emu_two_sum(b, c, &uh, &ul); emu_two_sum(a, uh, &th, &tl);
  double v = emu_ro_sum(tl, ul); return th + v; }
static inline double emu_fma(double a, double b, double c){
  double p, e; emu_two_prod(a, b, &p, &e); return emu_rn3(p, e, c); }
/* erf's tiny path ends with fma(l, 2^-106, y), whose product can be subnormal, where Dekker's product is not exact.
   RN(y + l*2^-106) exactly instead: Y = y*2^106 and Y + l = sh + sl exactly (TwoSum); a normal result is sh*2^-106
   (sh is RN(Y + l), and the scaling is exact); a subnormal one is N*2^-1074 with N = RN(Y + l) at the subnormal grid
   2^-968 of the scaled sum, found as RN(M + sh*2^968 + sl*2^968) - M with M = 2^52, so that M + t stays in
   [2^52, 2^53), where the grid is 1, for every subnormal result (one exact rn3 of full-precision numbers). */
static inline double emu_fma_tiny(double l, double y){
  double sh, sl; emu_two_sum(y * 0x1p106, l, &sh, &sl);
  double ash = sh < 0 ? -sh : sh;
  if (ash >= 0x1p-916) return sh * 0x1p-106;
  const double M = 0x1p52;
  double m = sh < 0 ? -M : M;
  double z = emu_rn3(m, sh * 0x1p968, sl * 0x1p968);
  return (z - m) * 0x1p-1074; }
