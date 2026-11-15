import sys, math, struct
from fractions import Fraction
from math import comb
def f(h): return struct.unpack('<d', struct.pack('<Q', int(h,16)))[0]
stats = {'P': [0,0,0.0,0.0,None], 'Q': [0,0,0.0,0.0,None]}  # n, notCR, max, sum, worst
for line in open(sys.argv[1]):
    a,b,k,ph,qh = line.split(); a=int(a); b=int(b); k=int(k)
    n = a+b-1; D = 1<<20
    num = sum(comb(n,j) * k**j * (D-k)**(n-j) for j in range(a, n+1))
    P = Fraction(num, D**n); Q = 1 - P
    for name, exact, got in (('P', P, f(ph)), ('Q', Q, f(qh))):
        cr = float(exact)
        if cr == 0.0 or abs(cr) < 2.2250738585072014e-308: continue
        e = float(abs(Fraction(got) - exact) / Fraction(math.ulp(cr)))
        s = stats[name]; s[0]+=1; s[1]+= (got != cr); s[3]+=e
        if e > s[2]: s[2]=e; s[4]=(a,b,k/D)
for name,s in stats.items():
    print(f"beta_inc_{name:<6} n={s[0]:5d} max={s[2]:.4g} ulp mean={s[3]/s[0]:.3g} notCR={100*s[1]/s[0]:.2f}% worst(a,b,x)={s[4]}")
