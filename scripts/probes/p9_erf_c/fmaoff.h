/* fmaoff.h — probe P9 stage 1: force-included (-include) into CORE-MATH's erf.c and erfc.c, so that every
   __builtin_fma there runs as emu.h's exact emulation, built with -mno-fma. */
#include "emu.h"
#define __builtin_fma(a, b, c) emu_fma((a), (b), (c))
