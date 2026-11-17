"""NIST StRD (port plan §3.6, decision D4): the univariate, one-way ANOVA and linear least squares sets, NIST's files
as NIST publishes them (public domain), each pinned by its sha256 here. They are committed in
scripts/golden/strd.tar.xz (12 KB), and unpacked into build/strd/: NIST's server refuses GitHub's runners (HTTP 403),
so CI can't fetch them, and the corpus can't depend on the network. The emitters replay each set through the port as
golden rows (the replay proves the port computes what 1.3.0's model does, bit for bit), and `report` measures that
model against NIST's certified values as LRE, the number of correct significant digits: D4 keeps Rust's operation
order in 2.0 and files any shortfall as a 2.1 candidate, with these numbers.

    python3 -I -B scripts/golden/strd.py .           # unpack what is missing, verify, print the LRE table

The archive, rebuilt from NIST's files in build/strd/ (BASE + univ/data/, anova/ or lls/data/LINKS/DATA/ + <set>.dat):

    tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner --mode=u=rwX,go=rX -C build -cf - strd \\
        | xz -9e -T1 > scripts/golden/strd.tar.xz
"""
import hashlib
import math
import sys
import tarfile
from fractions import Fraction
from pathlib import Path

BASE = "https://www.itl.nist.gov/div898/strd/"
UNIVARIATE = ["PiDigits", "Lottery", "Lew", "Mavro", "Michelso", "NumAcc1", "NumAcc2", "NumAcc3", "NumAcc4"]
ANOVA = ["SiRstv", "SmLs01", "SmLs02", "SmLs03", "AtmWtAg", "SmLs04", "SmLs05", "SmLs06", "SmLs07", "SmLs08",
         "SmLs09"]
# Linear least squares (M5 part d): the sets 1.3.0's fits can take, y on x alone. NoInt1/2 need a fit without an
# intercept and Longley a multiple regression, neither of which 1.3.0 has.
REGRESSION = ["Norris", "Pontius", "Filip", "Wampler1", "Wampler2", "Wampler3", "Wampler4", "Wampler5"]
SHA256 = {
    "AtmWtAg": "5a1083c9062d3966f244bffa376fc9b033244abf86a0d0dd016af39cd71fb8a7",
    "Lew": "081b231f686d01fa00247b8072909206947c8321bd62e060d9aaae5f4866c44e",
    "Lottery": "956cb33392ee28683aa7978326fc356d1461e4a3b0e5a69d317fd481d02d6255",
    "Mavro": "6b967538d6a607fdf30df054c6fbf4ca6bfb4fa6dc2ecbe613d4f93237a0b1dd",
    "Michelso": "68cad435641539ca020adafb4c9a6b7fcc4fefe1d6ab8f2af8fb858d4f1b12ab",
    "NumAcc1": "ce7c7a0fc6d7cedd0d5e1f9911afb9337544be6836730beb9968ca0a4571d375",
    "NumAcc2": "5be29522dfbe299541f0ec98ec1c22a51a5587bf06561ba21a92c209fb7ef0f9",
    "NumAcc3": "66df1b96596735936f37fd889e7c1c6ce998eb2b523bfc46c98ec1737e992aec",
    "NumAcc4": "ca310dc767f5130f980f8280bbe69e26ba414a4d85a89fc11b6c767b828e5fe4",
    "PiDigits": "e9421a2f4c0a31c505099332b4d22e507e808953c3c57bddae9226f25587d7d2",
    "SiRstv": "5a4f02d7f256e18c30dcad1b81e0260bb810b5ac113f6107e06dee168d07663e",
    "SmLs01": "ad28cac0d4fb79ffea2522b55349083bdea10f12a6f8c46cfbabcccdb4d6b896",
    "SmLs02": "708e32eaf36769abe7533e7dcd1b6dc67091b60df938dd37c4f0e65660ef7b0b",
    "SmLs03": "e18404d0628b78670580b230a3a3b6cd997d959d00f92a2883a4da942b19377a",
    "SmLs04": "1308eb1581f28ce6c9de4ca9520fcc729f916c34f1f9dee10fb6e6581e5b4ce7",
    "SmLs05": "7b1333e6e56ced74145096df774cfb1f6305213cb28f9a8222c418ca85d0c296",
    "SmLs06": "0c0f09a2df05f2e9db5324a6aaeed85647651e874e9f5c3892b8f29fc4a200f7",
    "SmLs07": "fd95c510aa8b3ce7f67ddd32846ccead7d9abae441790336c6fb0fcb0ae3558f",
    "SmLs08": "87dd38dfffeb65d39dbe3e8ba73bd16644fc396c1aad03fc0295225f3ba6aef9",
    "SmLs09": "f4688c5a0d5690fc86fa27e0c5a265eef0b24641e74af27aa72d9615084dd13f",
    "Norris": "5ab6906c68a9c4b2eb1661c0cd7d2a09f5a469a6d4e05b6b691f190f5f7b944f",
    "Pontius": "de60baa5dc66fd804f9316f895bc5aa2b8f791c09d1ecc1ec39f8b199b8c15c7",
    "Filip": "403b34689e401d915cb28bd69311d3ca5c3007d88d3b670b00e3203042315072",
    "Wampler1": "8bcd6f00dbe8c307ebc5c971e12b950eb3cdb75afbc2b38573082b5fcb0d7fbc",
    "Wampler2": "01ba5f08b5b9f84431f3754216e006939a80264a1caa69fe08b3cbc543f9ee3e",
    "Wampler3": "c5b025f6ebf0365de92d80e478ad826b01a6dfdafb71964ea8bae7eec993ce53",
    "Wampler4": "dcff32075a52ecf31af70dad340776237544b1a29b5034a40f8b3e6b607cf9c1",
    "Wampler5": "c4e260a6638db61287bfea75a2dd3b324c86c5d40931ddc6d8fab0c0305ffa39",
}
ROOT = Path(__file__).resolve().parents[2]


def path(name, root=ROOT):
    """The set's file in build/strd/, unpacked from scripts/golden/strd.tar.xz if it isn't there yet; a hash mismatch
    refuses."""
    d = root / "build" / "strd"
    d.mkdir(parents=True, exist_ok=True)
    p = d / f"{name}.dat"
    if not p.exists():
        with tarfile.open(root / "scripts" / "golden" / "strd.tar.xz", "r:xz") as t:
            p.write_bytes(t.extractfile(f"strd/{name}.dat").read())
    got = hashlib.sha256(p.read_bytes()).hexdigest()
    if got != SHA256[name]:
        raise SystemExit(f"strd: {p} has sha256 {got}, expected {SHA256[name]}")
    return p


def univariate(name, root=ROOT):
    """(data, certified {mean, sd, r1}) of a univariate set; the data as the doubles their decimals round to."""
    lines = path(name, root).read_text(encoding="ascii").splitlines()
    cert = {}
    for l in lines:
        if "ybar:" in l:
            cert["mean"] = l.split("ybar:")[1].split()[0]
        if " s:" in l and "Standard Deviation" in l:
            cert["sd"] = l.split(" s:")[1].split()[0]
        if "r(1):" in l:
            cert["r1"] = l.split("r(1):")[1].split()[0]
    first = next(int(l.split("lines")[1].split("to")[0]) for l in lines if l.strip().startswith("Data") and "lines" in l)
    data = [float(l) for l in lines[first - 1:] if l.strip()]
    return data, cert


def anova(name, root=ROOT):
    """(groups, certified {ssb, ssw, msb, msw, f, r2, rsd, dfb, dfw}) of a one-way ANOVA set, the groups in treatment
    order."""
    lines = path(name, root).read_text(encoding="ascii").splitlines()
    cert = {}
    for l in lines:
        w = l.split()
        if l.startswith("Between ") and len(w) == 6:
            cert["dfb"], cert["ssb"], cert["msb"], cert["f"] = int(w[2]), w[3], w[4], w[5]
        if l.startswith("Within ") and len(w) == 5:
            cert["dfw"], cert["ssw"], cert["msw"] = int(w[2]), w[3], w[4]
        if "Certified R-Squared" in l:
            cert["r2"] = w[-1]
        if "Standard Deviation" in l and "Residual" not in l and cert.get("rsd") is None and "Certified" not in l:
            cert["rsd"] = w[-1]
    first = next(int(l.split("lines")[1].split("to")[0]) for l in lines if l.strip().startswith("Data") and "lines" in l)
    groups = {}
    for l in lines[first - 1:]:
        w = l.split()
        if len(w) == 2:
            groups.setdefault(int(w[0]), []).append(float(w[1]))
    return [groups[k] for k in sorted(groups)], cert


def regression(name, root=ROOT):
    """(x, y, certified {coefficients B0 … Bk as decimals, r2}) of a linear least squares set: k is the degree."""
    lines = path(name, root).read_text(encoding="ascii").splitlines()
    cert, coefs = {}, []
    for l in lines:
        w = l.split()
        if len(w) >= 2 and w[0].startswith("B") and w[0][1:].isdigit():
            coefs.append(w[1])
        if l.strip().startswith("R-Squared"):
            cert["r2"] = w[-1]
    cert["coefficients"] = coefs
    span = next(l for l in lines if "Data" in l and "(lines" in l)
    first = int(span.split("(lines")[1].split("to")[0])
    x, y = [], []
    for l in lines[first - 1:]:
        w = l.split()
        if len(w) == 2:
            y.append(float(w[0]))
            x.append(float(w[1]))
    return x, y, cert


def lre(x, c):
    """The log relative error of x against the certified decimal c: −log10(|x − c|/|c|), or −log10|x − c| at c = 0;
    15 when they agree to the certified value's 15 digits or better, as NIST's convention caps it."""
    cf = Fraction(c)
    if isinstance(x, float) and not math.isfinite(x):
        return 0.0
    d = abs(Fraction(x) - cf)
    if d == 0:
        return 15.0
    v = float(d / abs(cf)) if cf != 0 else float(d)
    return min(15.0, -math.log10(v))


def sd_from_variance(v, n):
    """The certified sample standard deviation's counterpart from the port's population variance, √(v·n/(n − 1)),
    exactly (the √ by a rational Newton step to far below an ulp)."""
    t = Fraction(v) * n / (n - 1)
    x = Fraction(math.sqrt(float(t)))
    for _ in range(3):
        x = (x + t / x) / 2
    return x


def report(root=ROOT):
    """The LRE of every certified value, from validate.py's models (the port's values, as the replay proves)."""
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from emit.models import V
    rows = []
    for name in UNIVARIATE:
        data, cert = univariate(name, root)
        m = V.ds_mean(data)
        v = V.ds_variance(data)
        s = sd_from_variance(v, len(data))
        rows.append((name, "mean", lre(m, cert["mean"])))
        rows.append((name, "sd (from the variance)", lre(float(s), cert["sd"]) if s else 0.0))
        r1 = V.ts_acf(data, 1)[0]
        rows.append((name, "r(1)", lre(r1, cert["r1"])))
    L = V.dist_prims(root)[2]
    for name in ANOVA:
        groups, cert = anova(name, root)
        out = V.hy_anova(L, groups, [0.05])
        ssb, ssw, sst, dfb, dfw, msb, msw, f_ = out[:8]
        rows.append((name, "SS between", lre(ssb, cert["ssb"])))
        rows.append((name, "SS within", lre(ssw, cert["ssw"])))
        rows.append((name, "MS between", lre(msb, cert["msb"])))
        rows.append((name, "MS within", lre(msw, cert["msw"])))
        rows.append((name, "F", lre(f_, cert["f"])))
        rows.append((name, "R²", lre(ssb / sst, cert["r2"])))
        if dfb != cert["dfb"] or dfw != cert["dfw"]:
            rows.append((name, "df", 0.0))
    for name in REGRESSION:
        x, y, cert = regression(name, root)
        k = len(cert["coefficients"]) - 1
        fit = V.rg_poly(x, y, k)
        if isinstance(fit, str):
            rows.append((name, f"degree {k}: {fit}", 0.0))
            continue
        coefs, _, r2 = fit
        worst = min(lre(c, b) for c, b in zip(coefs, cert["coefficients"]))
        rows.append((name, f"degree {k}, worst coefficient", worst))
        rows.append((name, f"degree {k}, R²", lre(r2, cert["r2"])))
        if k == 1:
            lin = V.rg_linear(x, y)
            rows.append((name, "linear fit, slope", lre(lin[0], cert["coefficients"][1])))
            rows.append((name, "linear fit, intercept", lre(lin[1], cert["coefficients"][0])))
    return rows


if __name__ == "__main__":
    root = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT
    for name, what, digits in report(root):
        print(f"{name:10s} {what:24s} LRE {digits:5.1f}")
