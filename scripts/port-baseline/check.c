// Reads "name x [y] rust" (hex bit patterns), computes the exact value with MPFR,
// reports per function: n, max ulp error, mean ulp error, fraction not correctly rounded,
// and the worst input. A name ending "_mixed" is measured in ulps of max(|value|, 1).
// Probe P9's incomplete gamma rows ("gref_p…", "gref_q…": "name a x result") use gref() below, a 320-bit series and
// continued fraction, since mpfr_gamma_inc takes 0.05-0.8 s a call near a = 1e5. They count zero and subnormal
// results in units of 2^-1074 (ADR 009). "gref_vs_mpfr a x 0" rows report gref's own relative error against
// mpfr_gamma_inc at 1,024 bits, in units of 2^-53.
// Probe P9's incomplete beta rows ("bref_i…" for I_x(a, b), "bref_c…" for 1 - I_x(a, b): "name a b x y result") use
// bref() below, a 320-bit continued fraction, since MPFR has no incomplete beta; the smaller of x and y is exact and
// the other 1 - it. "bref_vs_exact a b x y <decimal>" rows report bref's relative error against an exact value given
// as a decimal string, in units of 2^-53.
#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include <math.h>
#include <mpfr.h>
typedef struct { char name[48]; long n, ncr, skipped; double maxu, sumu, worst_x, worst_y, worst_z; } Stat;
static Stat st[128]; static int nst;
static Stat *get(const char *nm){ for(int i=0;i<nst;i++) if(!strcmp(st[i].name,nm)) return &st[i]; strcpy(st[nst].name,nm); return &st[nst++]; }
static double fromh(const char *s){ uint64_t u; sscanf(s,"%lx",&u); double d; memcpy(&d,&u,8); return d; }
// P(a, x) and Q(a, x) at prec = 320 bits: pre = x^a e^-x / Γ(a); for x <= a or x < 1, P = pre·Σ_{n>=0}
// x^n/(a(a+1)…(a+n)) (positive terms, stopped under 2^-(prec + 10) of the sum) and Q = 1 - P; otherwise Q =
// pre·(Legendre's continued fraction by modified Lentz, stopped when a step changes it by under 2^-310) and P = 1 - Q.
// The complement is taken from the exact product (2·prec bits). Where x <= a, Q is at least about 0.3, so it keeps
// ~318 bits. For x < 1 (M2 part b4: the fraction converges too slowly there for a small shape, and never in practice
// for a tiny one), Q is at least about 0.2a, and below a = 2^-10, where that loses more than 12 bits, prec grows by
// log2(1/a) + 64.
static void gref(mpfr_t P, mpfr_t Q, double ad, double xd){
  mpfr_t a,x,pre,t,s,term,h,b,c,d,an,del,tiny,lim,w; int sg;
  long prec=320;
  if(ad<0x1p-10) prec+=64+(long)ceil(-log2(ad));
  mpfr_inits2(prec,a,x,pre,t,s,term,h,b,c,d,an,del,tiny,lim,(mpfr_ptr)0);
  mpfr_init2(w,2*prec);
  mpfr_set_d(a,ad,MPFR_RNDN); mpfr_set_d(x,xd,MPFR_RNDN);
  mpfr_log(t,x,MPFR_RNDN); mpfr_mul(pre,t,a,MPFR_RNDN); mpfr_sub(pre,pre,x,MPFR_RNDN);
  mpfr_lgamma(t,&sg,a,MPFR_RNDN); mpfr_sub(pre,pre,t,MPFR_RNDN); mpfr_exp(pre,pre,MPFR_RNDN);
  if(xd<=ad || xd<1.0){
    mpfr_ui_div(term,1,a,MPFR_RNDN); mpfr_set(s,term,MPFR_RNDN);
    for(unsigned long n=1;;n++){
      mpfr_add_ui(t,a,n,MPFR_RNDN); mpfr_div(t,x,t,MPFR_RNDN);
      mpfr_mul(term,term,t,MPFR_RNDN); mpfr_add(s,s,term,MPFR_RNDN);
      mpfr_mul_2si(lim,s,-(prec+10),MPFR_RNDN); if(mpfr_cmp(term,lim)<0) break;
    }
    mpfr_mul(w,s,pre,MPFR_RNDN); mpfr_set(P,w,MPFR_RNDN); mpfr_ui_sub(Q,1,w,MPFR_RNDN);
  } else {
    mpfr_set_ui_2exp(tiny,1,-20000,MPFR_RNDN);
    mpfr_add_ui(b,x,1,MPFR_RNDN); mpfr_sub(b,b,a,MPFR_RNDN);
    mpfr_set_ui_2exp(c,1,20000,MPFR_RNDN); mpfr_ui_div(d,1,b,MPFR_RNDN); mpfr_set(h,d,MPFR_RNDN);
    for(unsigned long i=1;;i++){
      mpfr_ui_sub(an,i,a,MPFR_RNDN); mpfr_mul_ui(an,an,i,MPFR_RNDN); mpfr_neg(an,an,MPFR_RNDN);
      mpfr_add_ui(b,b,2,MPFR_RNDN);
      mpfr_mul(d,an,d,MPFR_RNDN); mpfr_add(d,d,b,MPFR_RNDN);
      mpfr_abs(t,d,MPFR_RNDN); if(mpfr_cmp(t,tiny)<0) mpfr_set(d,tiny,MPFR_RNDN);
      mpfr_div(t,an,c,MPFR_RNDN); mpfr_add(c,b,t,MPFR_RNDN);
      mpfr_abs(t,c,MPFR_RNDN); if(mpfr_cmp(t,tiny)<0) mpfr_set(c,tiny,MPFR_RNDN);
      mpfr_ui_div(d,1,d,MPFR_RNDN); mpfr_mul(del,d,c,MPFR_RNDN); mpfr_mul(h,h,del,MPFR_RNDN);
      mpfr_sub_ui(t,del,1,MPFR_RNDN); mpfr_abs(t,t,MPFR_RNDN); if(mpfr_cmp_d(t,0x1p-310)<0) break;
    }
    mpfr_mul(w,h,pre,MPFR_RNDN); mpfr_set(Q,w,MPFR_RNDN); mpfr_ui_sub(P,1,w,MPFR_RNDN);
  }
  mpfr_clears(a,x,pre,t,s,term,h,b,c,d,an,del,tiny,lim,w,(mpfr_ptr)0);
}
// x^a y^b / (a B(a, b)) times Lentz's continued fraction (Rust's even and odd steps), at 320 bits, stopped when a
// step changes it by under 2^-310: I_x(a, b) for x < (a+1)/(a+b+2).
static void bcf(mpfr_t r, mpfr_t a, mpfr_t b, mpfr_t x, mpfr_t y){
  mpfr_t t,u,pre,c,d,h,num,del,tiny; int sg;
  mpfr_inits2(320,t,u,pre,c,d,h,num,del,tiny,(mpfr_ptr)0);
  mpfr_set_ui_2exp(tiny,1,-20000,MPFR_RNDN);
  mpfr_log(t,x,MPFR_RNDN); mpfr_mul(pre,t,a,MPFR_RNDN);
  mpfr_log(t,y,MPFR_RNDN); mpfr_mul(t,t,b,MPFR_RNDN); mpfr_add(pre,pre,t,MPFR_RNDN);
  mpfr_lgamma(t,&sg,a,MPFR_RNDN); mpfr_sub(pre,pre,t,MPFR_RNDN);
  mpfr_lgamma(t,&sg,b,MPFR_RNDN); mpfr_sub(pre,pre,t,MPFR_RNDN);
  mpfr_add(u,a,b,MPFR_RNDN); mpfr_lgamma(t,&sg,u,MPFR_RNDN); mpfr_add(pre,pre,t,MPFR_RNDN);
  mpfr_log(t,a,MPFR_RNDN); mpfr_sub(pre,pre,t,MPFR_RNDN); mpfr_exp(pre,pre,MPFR_RNDN);
  mpfr_set_ui(c,1,MPFR_RNDN);
  mpfr_ui_sub(t,1,b,MPFR_RNDN); mpfr_mul(t,t,x,MPFR_RNDN); mpfr_add_ui(u,a,1,MPFR_RNDN); mpfr_mul(u,u,y,MPFR_RNDN);
  mpfr_add(d,t,u,MPFR_RNDN); mpfr_add_ui(u,a,1,MPFR_RNDN); mpfr_div(d,d,u,MPFR_RNDN);
  mpfr_ui_div(d,1,d,MPFR_RNDN); mpfr_set(h,d,MPFR_RNDN);
  for(unsigned long m=1;;m++){
    for(int odd=0;odd<2;odd++){
      if(!odd){ // m(b - m)x / ((a + 2m - 1)(a + 2m))
        mpfr_sub_ui(num,b,m,MPFR_RNDN); mpfr_mul_ui(num,num,m,MPFR_RNDN); mpfr_mul(num,num,x,MPFR_RNDN);
        mpfr_add_ui(t,a,2*m-1,MPFR_RNDN); mpfr_add_ui(u,a,2*m,MPFR_RNDN);
      } else { // -(a + m)(a + b + m)x / ((a + 2m)(a + 2m + 1))
        mpfr_add_ui(num,a,m,MPFR_RNDN); mpfr_add(t,a,b,MPFR_RNDN); mpfr_add_ui(t,t,m,MPFR_RNDN);
        mpfr_mul(num,num,t,MPFR_RNDN); mpfr_mul(num,num,x,MPFR_RNDN); mpfr_neg(num,num,MPFR_RNDN);
        mpfr_add_ui(t,a,2*m,MPFR_RNDN); mpfr_add_ui(u,a,2*m+1,MPFR_RNDN);
      }
      mpfr_mul(t,t,u,MPFR_RNDN); mpfr_div(num,num,t,MPFR_RNDN);
      mpfr_mul(d,num,d,MPFR_RNDN); mpfr_add_ui(d,d,1,MPFR_RNDN);
      mpfr_abs(t,d,MPFR_RNDN); if(mpfr_cmp(t,tiny)<0) mpfr_set(d,tiny,MPFR_RNDN);
      mpfr_div(t,num,c,MPFR_RNDN); mpfr_add_ui(c,t,1,MPFR_RNDN);
      mpfr_abs(t,c,MPFR_RNDN); if(mpfr_cmp(t,tiny)<0) mpfr_set(c,tiny,MPFR_RNDN);
      mpfr_ui_div(d,1,d,MPFR_RNDN); mpfr_mul(del,d,c,MPFR_RNDN); mpfr_mul(h,h,del,MPFR_RNDN);
    }
    mpfr_sub_ui(t,del,1,MPFR_RNDN); mpfr_abs(t,t,MPFR_RNDN); if(mpfr_cmp_d(t,0x1p-310)<0) break;
  }
  mpfr_mul(r,pre,h,MPFR_RNDN);
  mpfr_clears(t,u,pre,c,d,h,num,del,tiny,(mpfr_ptr)0);
}
// I_x(a, b) and 1 - I_x(a, b) at 320 bits, the smaller of xd and yd exact: the continued fraction on the side where it
// converges fast, the other value as 1 - it (which keeps at least ~270 bits on every row probe P9 makes).
static void bref(mpfr_t I, mpfr_t C, double ad, double bd, double xd, double yd){
  mpfr_t a,b,x,y,s,r;
  mpfr_inits2(320,a,b,x,y,s,r,(mpfr_ptr)0);
  mpfr_set_d(a,ad,MPFR_RNDN); mpfr_set_d(b,bd,MPFR_RNDN);
  if(xd<=yd){ mpfr_set_d(x,xd,MPFR_RNDN); mpfr_ui_sub(y,1,x,MPFR_RNDN); }
  else { mpfr_set_d(y,yd,MPFR_RNDN); mpfr_ui_sub(x,1,y,MPFR_RNDN); }
  mpfr_add_ui(s,a,1,MPFR_RNDN); mpfr_add(r,a,b,MPFR_RNDN); mpfr_add_ui(r,r,2,MPFR_RNDN); mpfr_div(s,s,r,MPFR_RNDN);
  if(mpfr_cmp(x,s)<0){ bcf(I,a,b,x,y); mpfr_ui_sub(C,1,I,MPFR_RNDN); }
  else { bcf(C,b,a,y,x); mpfr_ui_sub(I,1,C,MPFR_RNDN); }
  mpfr_clears(a,b,x,y,s,r,(mpfr_ptr)0);
}
int main(void){
  mpfr_set_default_prec(1024);
  mpfr_t v,a,b,t,gp,gq; mpfr_inits2(1024,v,a,b,t,gp,gq,(mpfr_ptr)0);
  char nm[48], s1[32], s2[32], s3[32], s4[32]; static char s5[512]; char line[1024];
  while(fgets(line,sizeof line,stdin)){
    int k=sscanf(line,"%47s %31s %31s %31s %31s %511s",nm,s1,s2,s3,s4,s5);
    int gr=!strncmp(nm,"gref_p",6)||!strncmp(nm,"gref_q",6);
    int br=!strncmp(nm,"bref_i",6)||!strncmp(nm,"bref_c",6);
    double x=fromh(s1), y=0, r;
    if(k==4){ y=fromh(s2); r=fromh(s3);} else r=fromh(s2);
    if(!strcmp(nm,"bref_vs_exact")){
      // bref's I against an exact value (a decimal string): the relative error in units of 2^-53.
      double ba=fromh(s1), bb=fromh(s2), bx=fromh(s3), by=fromh(s4);
      bref(gp,gq,ba,bb,bx,by); mpfr_set_str(v,s5,10,MPFR_RNDN);
      mpfr_sub(t,gp,v,MPFR_RNDN); mpfr_div(t,t,v,MPFR_RNDN); mpfr_abs(t,t,MPFR_RNDN); mpfr_mul_2si(t,t,53,MPFR_RNDN);
      double e=mpfr_get_d(t,MPFR_RNDN); Stat *s=get(nm); s->n++; s->sumu+=e;
      if(e>s->maxu){ s->maxu=e; s->worst_x=ba; s->worst_y=bb; }
      continue;
    }
    if(br){
      // "name a b x y result": the reference once per input, as for gref.
      static double ka=-1, kb=-1, kx=-1, ky=-1;
      double ba=fromh(s1), bb=fromh(s2), bx=fromh(s3), by=fromh(s4); r=fromh(s5);
      if(ba!=ka||bb!=kb||bx!=kx||by!=ky){ bref(gp,gq,ba,bb,bx,by); ka=ba; kb=bb; kx=bx; ky=by; }
      Stat *s=get(nm);
      double cr=mpfr_get_d(nm[5]=='i'?gp:gq,MPFR_RNDN);
      if(!isfinite(cr) || isnan(r)){ s->skipped++; continue; }
      double u=nextafter(fabs(cr),INFINITY)-fabs(cr);
      mpfr_set_d(t,r,MPFR_RNDN); mpfr_sub(t,t,nm[5]=='i'?gp:gq,MPFR_RNDN); mpfr_abs(t,t,MPFR_RNDN);
      double e=mpfr_get_d(t,MPFR_RNDN)/u;
      s->n++; s->sumu+=e; if(r!=cr) s->ncr++;
      if(e>s->maxu){ s->maxu=e; s->worst_x=ba; s->worst_y=bb; s->worst_z=(bx<=by?bx:-by); }
      continue;
    }
    mpfr_set_d(a,x,MPFR_RNDN); mpfr_set_d(b,y,MPFR_RNDN);
    // M1's correct-rounding rows, "cr:<fn>/<set>" (scripts/crverify/): the function before the "/" picks the
    // reference, and zero and subnormal results count in units of 2^-1074 (ADR 009). The full name keeps the stats.
    char full[48]; int crv=0;
    if(!strncmp(nm,"cr:",3)){
      crv=1; strcpy(full,nm);
      char *sl=strchr(nm+3,'/'); size_t k=sl?(size_t)(sl-(nm+3)):strlen(nm+3);
      memmove(nm,nm+3,k); nm[k]=0;
    }
    int ok=1;
    if(!strcmp(nm,"exp")) mpfr_exp(v,a,MPFR_RNDN);
    else if(!strcmp(nm,"log")) mpfr_log(v,a,MPFR_RNDN);
    else if(!strcmp(nm,"log1p")) mpfr_log1p(v,a,MPFR_RNDN);
    else if(!strcmp(nm,"expm1")) mpfr_expm1(v,a,MPFR_RNDN);
    else if(!strcmp(nm,"sin")) mpfr_sin(v,a,MPFR_RNDN);
    else if(!strcmp(nm,"cos")) mpfr_cos(v,a,MPFR_RNDN);
    else if(!strcmp(nm,"tan")) mpfr_tan(v,a,MPFR_RNDN);
    else if(!strcmp(nm,"atan")) mpfr_atan(v,a,MPFR_RNDN);
    else if(!strcmp(nm,"log2")) mpfr_log2(v,a,MPFR_RNDN);
    else if(!strcmp(nm,"pow")) mpfr_pow(v,a,b,MPFR_RNDN);
    else if(!strncmp(nm,"erfc",4)) mpfr_erfc(v,a,MPFR_RNDN);  // "erfc", and probe P9's "erfc/set", "erfc_acc/set"
    else if(!strncmp(nm,"erf",3)) mpfr_erf(v,a,MPFR_RNDN);    // "erf", "erf/set", "erf_acc/set"
    else if(!strncmp(nm,"ncdf",4)){ mpfr_sqrt_ui(t,2,MPFR_RNDN); mpfr_div(t,a,t,MPFR_RNDN); mpfr_neg(t,t,MPFR_RNDN); mpfr_erfc(v,t,MPFR_RNDN); mpfr_div_ui(v,v,2,MPFR_RNDN); }
    else if(!strncmp(nm,"lgamma",6)){ int sg; mpfr_lgamma(v,&sg,a,MPFR_RNDN); }
    else if(!strcmp(nm,"gamma_q")||!strcmp(nm,"gamma_p")){
      // a = shape (x field), b = x (y field): Q = Gamma(a,x)/Gamma(a)
      mpfr_gamma_inc(v,a,b,MPFR_RNDN); mpfr_gamma(t,a,MPFR_RNDN); mpfr_div(v,v,t,MPFR_RNDN);
      if(!strcmp(nm,"gamma_p")) mpfr_ui_sub(v,1,v,MPFR_RNDN);
    } else if(gr){
      // One pair's rows arrive together: compute the reference once per pair.
      static double gx=-1, gy=-1;
      if(x!=gx||y!=gy){ gref(gp,gq,x,y); gx=x; gy=y; }
      mpfr_set(v,nm[5]=='p'?gp:gq,MPFR_RNDN);
    }
    else if(!strcmp(nm,"gref_vs_mpfr")){
      // gref's Q against mpfr_gamma_inc's at 1,024 bits: the relative error in units of 2^-53.
      gref(gp,gq,x,y); mpfr_gamma_inc(v,a,b,MPFR_RNDN); mpfr_gamma(t,a,MPFR_RNDN); mpfr_div(v,v,t,MPFR_RNDN);
      mpfr_sub(t,gq,v,MPFR_RNDN); mpfr_div(t,t,v,MPFR_RNDN); mpfr_abs(t,t,MPFR_RNDN); mpfr_mul_2si(t,t,53,MPFR_RNDN);
      double e=mpfr_get_d(t,MPFR_RNDN); Stat *s=get(nm); s->n++; s->sumu+=e;
      if(e>s->maxu){ s->maxu=e; s->worst_x=x; s->worst_y=y; }
      continue;
    } else ok=0;
    if(crv) strcpy(nm,full);
    Stat *s=get(nm);
    if(!ok){ s->skipped++; continue; }
    double cr=mpfr_get_d(v,MPFR_RNDN);
    // Probe P9's erf/erfc and normal_cdf rows ("erf/…", "erfc_acc/…", "ncdf_core/t1", …) count zero and subnormal
    // results in units of 2^-1074 (ADR 009), as gref's do; every other row keeps the baseline's rule of skipping them.
    int p9e=crv || (strchr(nm,'/')!=NULL && (!strncmp(nm,"erf",3) || !strncmp(nm,"ncdf",4)));
    // A cr: row with a non-finite reference or result is compared too: right when both are the same infinity or both
    // NaN (0 ulp), wrong otherwise (counted as +∞ ulp), so overflow and the invalid cases are checked, not skipped.
    // So is a gref row (M2 part b4, whose first run skipped a NaN P at a = 1e-300): P and Q are always finite.
    if((crv || gr) && (!isfinite(cr) || !isfinite(r))){
      int same=(isnan(cr) && isnan(r)) || (isinf(cr) && cr==r);
      s->n++;
      if(!same){ s->ncr++; if(INFINITY>s->maxu){ s->maxu=INFINITY; s->worst_x=x; s->worst_y=y; } }
      continue;
    }
    if(!isfinite(cr) || (!gr && !p9e && fabs(cr) < 2.2250738585072014e-308) || isnan(r)){ s->skipped++; continue; }
    double u=nextafter(fabs(cr),INFINITY)-fabs(cr);
    // A name ending "_mixed" is measured in ulps of max(|cr|, 1): ADR 009's bound for lnΓ at x < 0 (probe P9).
    size_t nl=strlen(nm);
    if(nl>6 && !strcmp(nm+nl-6,"_mixed") && u<2.220446049250313e-16) u=2.220446049250313e-16;
    mpfr_set_d(t,r,MPFR_RNDN); mpfr_sub(t,t,v,MPFR_RNDN); mpfr_abs(t,t,MPFR_RNDN);
    double e=mpfr_get_d(t,MPFR_RNDN)/u;
    s->n++; s->sumu+=e; if(r!=cr) s->ncr++;
    if(e>s->maxu){ s->maxu=e; s->worst_x=x; s->worst_y=y; }
  }
  printf("%-14s %8s %12s %10s %9s  %s\n","function","n","max ulp","mean ulp","not CR","worst input");
  for(int i=0;i<nst;i++){ Stat*s=&st[i];
    printf("%-14s %8ld %12.4g %10.3g %8.4f%%  x=%.17g%s", s->name,s->n,s->maxu,s->n?s->sumu/s->n:0,100.0*s->ncr/(s->n?s->n:1),s->worst_x, "");
    if(!strncmp(s->name,"gamma",5)||!strncmp(s->name,"gref",4)||!strcmp(s->name,"pow")||!strncmp(s->name,"cr:pow",6))
      printf(" y=%.17g",s->worst_y);
    // A bref row's worst input is a (x=), b (b=) and then x, or −y when y was the smaller.
    if(!strncmp(s->name,"bref_i",6)||!strncmp(s->name,"bref_c",6)) printf(" b=%.17g x(−y)=%.17g",s->worst_y,s->worst_z);
    if(s->skipped) printf("  (skipped %ld)",s->skipped);
    printf("\n"); }
  return 0;
}
