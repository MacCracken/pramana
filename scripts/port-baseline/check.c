// Reads "name x [y] rust" (hex bit patterns), computes the exact value with MPFR,
// reports per function: n, max ulp error, mean ulp error, fraction not correctly rounded,
// and the worst input. A name ending "_mixed" is measured in ulps of max(|value|, 1).
// Probe P9's incomplete gamma rows ("gref_p…", "gref_q…": "name a x result") use gref() below, a 320-bit series and
// continued fraction, since mpfr_gamma_inc takes 0.05-0.8 s a call near a = 1e5. They count zero and subnormal
// results in units of 2^-1074 (ADR 009). "gref_vs_mpfr a x 0" rows report gref's own relative error against
// mpfr_gamma_inc at 1,024 bits, in units of 2^-53.
#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include <math.h>
#include <mpfr.h>
typedef struct { char name[48]; long n, ncr, skipped; double maxu, sumu, worst_x, worst_y; } Stat;
static Stat st[128]; static int nst;
static Stat *get(const char *nm){ for(int i=0;i<nst;i++) if(!strcmp(st[i].name,nm)) return &st[i]; strcpy(st[nst].name,nm); return &st[nst++]; }
static double fromh(const char *s){ uint64_t u; sscanf(s,"%lx",&u); double d; memcpy(&d,&u,8); return d; }
// P(a, x) and Q(a, x) at 320 bits: pre = x^a e^-x / Γ(a); for x <= a, P = pre·Σ_{n>=0} x^n/(a(a+1)…(a+n)) (positive
// terms, stopped under 2^-330 of the sum) and Q = 1 - P; for x > a, Q = pre·(Legendre's continued fraction by modified
// Lentz, stopped when a step changes it by under 2^-310) and P = 1 - Q. The complement is taken only where it is at
// least about 0.3, so it keeps ~318 bits.
static void gref(mpfr_t P, mpfr_t Q, double ad, double xd){
  mpfr_t a,x,pre,t,s,term,h,b,c,d,an,del,tiny,lim; int sg;
  mpfr_inits2(320,a,x,pre,t,s,term,h,b,c,d,an,del,tiny,lim,(mpfr_ptr)0);
  mpfr_set_d(a,ad,MPFR_RNDN); mpfr_set_d(x,xd,MPFR_RNDN);
  mpfr_log(t,x,MPFR_RNDN); mpfr_mul(pre,t,a,MPFR_RNDN); mpfr_sub(pre,pre,x,MPFR_RNDN);
  mpfr_lgamma(t,&sg,a,MPFR_RNDN); mpfr_sub(pre,pre,t,MPFR_RNDN); mpfr_exp(pre,pre,MPFR_RNDN);
  if(xd<=ad){
    mpfr_ui_div(term,1,a,MPFR_RNDN); mpfr_set(s,term,MPFR_RNDN);
    for(unsigned long n=1;;n++){
      mpfr_add_ui(t,a,n,MPFR_RNDN); mpfr_div(t,x,t,MPFR_RNDN);
      mpfr_mul(term,term,t,MPFR_RNDN); mpfr_add(s,s,term,MPFR_RNDN);
      mpfr_mul_2si(lim,s,-330,MPFR_RNDN); if(mpfr_cmp(term,lim)<0) break;
    }
    mpfr_mul(P,s,pre,MPFR_RNDN); mpfr_ui_sub(Q,1,P,MPFR_RNDN);
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
    mpfr_mul(Q,h,pre,MPFR_RNDN); mpfr_ui_sub(P,1,Q,MPFR_RNDN);
  }
  mpfr_clears(a,x,pre,t,s,term,h,b,c,d,an,del,tiny,lim,(mpfr_ptr)0);
}
int main(void){
  mpfr_set_default_prec(1024);
  mpfr_t v,a,b,t,gp,gq; mpfr_inits2(1024,v,a,b,t,gp,gq,(mpfr_ptr)0);
  char nm[48], s1[32], s2[32], s3[32]; char line[256];
  while(fgets(line,sizeof line,stdin)){
    int k=sscanf(line,"%47s %31s %31s %31s",nm,s1,s2,s3);
    int gr=!strncmp(nm,"gref_p",6)||!strncmp(nm,"gref_q",6);
    double x=fromh(s1), y=0, r;
    if(k==4){ y=fromh(s2); r=fromh(s3);} else r=fromh(s2);
    mpfr_set_d(a,x,MPFR_RNDN); mpfr_set_d(b,y,MPFR_RNDN);
    int ok=1;
    if(!strcmp(nm,"exp")) mpfr_exp(v,a,MPFR_RNDN);
    else if(!strcmp(nm,"log")) mpfr_log(v,a,MPFR_RNDN);
    else if(!strcmp(nm,"log1p")) mpfr_log1p(v,a,MPFR_RNDN);
    else if(!strcmp(nm,"expm1")) mpfr_expm1(v,a,MPFR_RNDN);
    else if(!strcmp(nm,"sin")) mpfr_sin(v,a,MPFR_RNDN);
    else if(!strcmp(nm,"cos")) mpfr_cos(v,a,MPFR_RNDN);
    else if(!strcmp(nm,"tan")) mpfr_tan(v,a,MPFR_RNDN);
    else if(!strcmp(nm,"pow")) mpfr_pow(v,a,b,MPFR_RNDN);
    else if(!strcmp(nm,"erf")) mpfr_erf(v,a,MPFR_RNDN);
    else if(!strcmp(nm,"erfc")) mpfr_erfc(v,a,MPFR_RNDN);
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
    Stat *s=get(nm);
    if(!ok){ s->skipped++; continue; }
    double cr=mpfr_get_d(v,MPFR_RNDN);
    if(!isfinite(cr) || (!gr && fabs(cr) < 2.2250738585072014e-308) || isnan(r)){ s->skipped++; continue; }
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
    if(!strncmp(s->name,"gamma",5)||!strncmp(s->name,"gref",4)||!strcmp(s->name,"pow")) printf(" y=%.17g",s->worst_y);
    if(s->skipped) printf("  (skipped %ld)",s->skipped);
    printf("\n"); }
  return 0;
}
