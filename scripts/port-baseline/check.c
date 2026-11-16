// Reads "name x [y] rust" (hex bit patterns), computes the exact value with MPFR,
// reports per function: n, max ulp error, mean ulp error, fraction not correctly rounded,
// and the worst input. A name ending "_mixed" is measured in ulps of max(|value|, 1).
#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include <math.h>
#include <mpfr.h>
typedef struct { char name[32]; long n, ncr, skipped; double maxu, sumu, worst_x, worst_y; } Stat;
static Stat st[64]; static int nst;
static Stat *get(const char *nm){ for(int i=0;i<nst;i++) if(!strcmp(st[i].name,nm)) return &st[i]; strcpy(st[nst].name,nm); return &st[nst++]; }
static double fromh(const char *s){ uint64_t u; sscanf(s,"%lx",&u); double d; memcpy(&d,&u,8); return d; }
int main(void){
  mpfr_set_default_prec(1024);
  mpfr_t v,a,b,t; mpfr_inits2(1024,v,a,b,t,(mpfr_ptr)0);
  char nm[32], s1[32], s2[32], s3[32]; char line[256];
  while(fgets(line,sizeof line,stdin)){
    int k=sscanf(line,"%31s %31s %31s %31s",nm,s1,s2,s3);
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
    } else ok=0;
    Stat *s=get(nm);
    if(!ok){ s->skipped++; continue; }
    double cr=mpfr_get_d(v,MPFR_RNDN);
    if(!isfinite(cr) || fabs(cr) < 2.2250738585072014e-308 || isnan(r)){ s->skipped++; continue; }
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
    if(!strncmp(s->name,"gamma",5)||!strcmp(s->name,"pow")) printf(" y=%.17g",s->worst_y);
    if(s->skipped) printf("  (skipped %ld)",s->skipped);
    printf("\n"); }
  return 0;
}
