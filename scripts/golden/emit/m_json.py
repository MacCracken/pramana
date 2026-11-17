"""`json.tsv`: serde_json 1.0.154's reading and writing of every shape 1.3.0's types serialize (M1 parts d3a–d4, e1; M3):
the value types, serde derive's struct rules on two test structs, 1.3.0's two enums, the error codec, SimpleRng and the
distributions (read through their validating constructors). Each read row is `serde_json::from_slice::<T>(document)`
as validate.py's serde_read models it: the value's tokens or `E:Deserialize`; each write row is serde_json's text as
serde_write and json_f64_text model it. Class A. validate.py writes json.map.tsv, the documents bayan refuses whole
where serde_json skips a member unchecked (ADR 007 row 16)."""
import urllib.parse
from decimal import Decimal

from .grids import INF, MAXF, MIN_POSITIVE, NAN, Rng
from .models import V
from .mvn_cases import mvn_cases, mvn_toks
from .tsv import Module, b, deser, e, err, f, from_bits, from_model, i, parsed, s, v

EPS = 2.220446049250313e-16
U64 = (1 << 64) - 1


def B(*xs):
    return [x.encode("utf-8") if isinstance(x, str) else x for x in xs]


COMMON = B("", " ", "\n", "null", "true", "false", "0", "1", "-1", "1.5", "\"x\"", "\"\"", "[]", "[1]", "[\"a\"]",
           "[[1]]", "{}", "{\"a\":1}", "nul", "nulll", "tru", "fals", "[", "]", "{", "}", "\"", ",", ":", "x", "1 x",
           "1,", "1 1", " 1 ", "\t1\r\n", "/*c*/1", "1//c", "[1]]", "[[1]", "\"x\"\"y\"") + [b"\xef\xbb\xbf1", b"1\x00",
                                                                                            b"\x0c1"]

NUMBERS = B(
    "0", "1", "-1", "-0", "-0.0", "0.0", "00", "01", "-01", "+1", ".5", "1.", "1.e3", "1e", "1e+", "-", "--1", "1.5.5",
    "1e3e3", "0x10", "1_000", "NaN", "Infinity", "-Infinity", "1.5", "1e3", "1E3", "1e+3", "1e-3", "1.0", "-0e5",
    "0e999", "1.5e300", "1e308", "1.7976931348623157e308", "1.7976931348623158e308", "1.7976931348623159e308",
    "2e308", "1e400", "-1e400", "1e-400", "4.9e-324", "5e-324", "2.4703282292062327e-324", "2.4703282292062328e-324",
    "2.2250738585072011e-308", "2.2250738585072012e-308", "2.2250738585072014e-308", "9007199254740992",
    "9007199254740993", "9007199254740995", "-9007199254740993", "9223372036854775807", "9223372036854775808",
    "9223372036854775809", "-9223372036854775807", "-9223372036854775808", "-9223372036854775809",
    "18446744073709551615", "18446744073709551616", "18446744073709551617", "-18446744073709551615",
    "-18446744073709551616", "100000000000000000000000", "123456789012345678901234567890e-10",
    "0.1000000000000000055511151231257827021181583404541015625",
    "1.00000000000000011102230246251565404236316680908203125",
    "1.00000000000000011102230246251565404236316680908203124",
    "1.00000000000000011102230246251565404236316680908203126", "0e18446744073709551616", "1e18446744073709551616",
    "1e-18446744073709551616", "-1e18446744073709551616", "1e9223372036854775807", "1e-9223372036854775808",
    "0.000000000000000000000000000000000000000001e42")

STRINGS = B(*['"abc"', '"\\b\\f\\n\\r\\t\\"\\\\\\/"', '"é"', '"É"', '"é"', '"😀"', '"😀"', '"😀"', '"\\ud800"',
              '"\\udc00"', '"\\ud800A"', '"\\ud800x"', '"\\udc00\\ud800"', '"\\ud800\\ud800"', '"\\u0000"', '"\\u001f"',
              '"\\u007f"', '"\uffff"', '"\\x"', '"\\u12"', '"\\u12G4"', '"\\U00e9"', '"\\"', '"a', '"\u2028"', '"\x7f"',
              '"\U0010ffff"']) + [b'"a\x01b"', b'"a\tb"', b'"a\nb"', b'"\x1f"', b'"\\\x01"', b'"a\xffb"', b'"\xc0\xaf"',
                                  b'"\xed\xa0\x80"', b'"\xf4\x90\x80\x80"', b'"\xe2\x82"', b'"\xf8\x88\x80\x80\x80"',
                                  b'"\x80"', b'"\xc2"', b'"\xe0\x80\xaf"']

POOL = ['a', 'Z', '0', ' ', '"', '\\', '/', '\n', '\t', '\x01', '\x1f', '\x7f', 'é', '€', '😀', '\uffff', '\U0010ffff',
        '\u2028', '\x00', '~', '%']
KERNELS = ["Gaussian", "Epanechnikov", "Uniform", "Triangular"]


def random_string(g):
    n = g.int(0, 12)
    return "".join(POOL[g.int(0, len(POOL) - 1)] for _ in range(n))


def utf16(c):
    cp = ord(c)
    if cp < 0x10000:
        return [cp]
    cp -= 0x10000
    return [0xD800 + (cp >> 10), 0xDC00 + (cp & 0x3FF)]


def escaped_string(g):
    d = bytearray(b'"')
    for _ in range(g.int(1, 8)):
        kind = g.int(0, 3)
        if kind == 0:
            cp = g.int(0x20, 0x7E)
        elif kind == 1:
            cp = g.int(0x80, 0xD7FF)
        elif kind == 2:
            cp = g.int(0xE000, 0xFFFF)
        else:
            cp = g.int(0x10000, 0x10FFFF)
        c = chr(cp)
        k = g.int(0, 2)
        if k == 0 and c not in ('"', '\\'):
            d += c.encode("utf-8")
        elif k == 1 and g.int(0, 1) == 0:
            for w in utf16(c):
                d += ("\\u%04X" % w).encode()
        else:
            for w in utf16(c):
                d += ("\\u%04x" % w).encode()
    d += b'"'
    return bytes(d)


def finite(g):
    while True:
        x = from_bits(g.word())
        if x == x and x not in (INF, -INF):
            return x


def midpoint(x):
    from .tsv import bits as tb
    bb = tb(x)
    exp = (bb >> 52) & 0x7FF
    frac = bb & ((1 << 52) - 1)
    m, e2 = (frac, -1074) if exp == 0 else (frac | (1 << 52), exp - 1075)
    n = 2 * m + 1
    k = e2 - 1
    if k >= 0:
        n <<= k
        point = 0
    else:
        n *= 5 ** (-k)
        point = -k
    d = str(n)
    return d.rstrip("0"), len(d) - 1 - point


def sci(digits, e10):
    return f"{digits}e{e10}" if len(digits) == 1 else f"{digits[0]}.{digits[1:]}e{e10}"


def rust_lower_exp(x):
    """Rust's `{:e}` for a finite f64: the shortest digits, d.ddde±x with no `+`."""
    sign = "-" if str(x).startswith("-") else ""
    if x == 0.0:
        return sign + "0e0"
    t = Decimal(repr(abs(x)))
    _, digits, exp = t.as_tuple()
    ds = "".join(map(str, digits)).rstrip("0") or "0"
    e10 = len(digits) - 1 + exp
    return sign + sci(ds, e10)


def rust_exp_prec(x, p):
    """Rust's `{:.Pe}`: the exact decimal rounded to P digits after the point (ties to even)."""
    t = format(Decimal(x), f".{p}e")
    mant, ex = t.split("e")
    return f"{mant}e{int(ex)}"


def json_text(x):
    return V.json_f64_text(x).encode()


def floats():
    g = Rng(0x6A09E667F3BCC908)
    out = [json_text(finite(g)) for _ in range(600)]
    for _ in range(200):
        out.append(rust_lower_exp(finite(g)).encode())
        out.append(rust_exp_prec(finite(g), 25).encode())
    for _ in range(400):
        nd = g.int(1, 30)
        t = "-" if g.int(0, 3) == 0 else ""
        t += chr(ord("1") + g.int(0, 8))
        if nd > 1:
            t += "." + "".join(chr(ord("0") + g.int(0, 9)) for _ in range(1, nd))
        t += f"e{g.int(-345, 312)}"
        out.append(t.encode())
    for k in range(120):
        if k % 6 == 0:
            x = from_bits(g.int(1, (1 << 52) - 1))
        else:
            x = from_bits((g.word() & 0x7FEFFFFFFFFFFFFF) | 0x0010000000000000)
        digits, e10 = midpoint(x)
        out.append(sci(digits, e10).encode())
        if digits.endswith("5"):
            out.append(sci(digits[:-1] + "49999", e10).encode())
        out.append(sci(digits + "000001", e10).encode())
    return out


# ── serde_json's writing of Python values, compact and pretty ──
# A value: ("f", x) | ("u", n) | ("b", bool) | ("s", str) | ("unit", name) | ("list", [...]) | ("obj", [(key, value)]).

def ser(val, pretty=False, level=0):
    k = val[0]
    if k == "f":
        return V.json_f64_text(val[1])
    if k == "u":
        return str(val[1] % (1 << 64))
    if k == "b":
        return "true" if val[1] else "false"
    if k == "s":
        return V.serde_escape(val[1].encode("utf-8")).decode("utf-8")
    if k == "unit":
        return '"%s"' % val[1]
    ind = "  "
    if k == "list":
        items = [ser(x, pretty, level + 1) for x in val[1]]
        if not items:
            return "[]"
        if not pretty:
            return "[" + ",".join(items) + "]"
        return "[\n" + ",\n".join(ind * (level + 1) + it for it in items) + "\n" + ind * level + "]"
    items = [(key, ser(x, pretty, level + 1)) for key, x in val[1]]
    if not items:
        return "{}"
    if not pretty:
        return "{" + ",".join('"%s":%s' % (key, t) for key, t in items) + "}"
    return "{\n" + ",\n".join(ind * (level + 1) + '"%s": %s' % (key, t) for key, t in items) + "\n" + ind * level + "}"


def fl(xs):
    return ("list", [("f", x) for x in xs])


def read(m, fn, doc, ty=None, code_only=False):
    ty = ty or fn[len("json::"):]
    want = V.serde_read(ty, doc)
    if want == ["E:Deserialize"]:
        m.row(fn, "A", [s(doc)], [deser()])
        return
    if code_only:
        want = want[:1]
    m.row(fn, "A", [s(doc)], from_model(want))


def write(m, ty, ins):
    m.row(f"json::write::{ty}", "A", ins, [s(V.serde_write(ty, iter(parsed(ins))))])


def wrap(open_, elems, close):
    return open_.encode() + b",".join(elems) + close.encode()


def obj(members):
    return b"{" + b",".join(b'"' + k.encode("utf-8") + b'":' + val for k, val in members) + b"}"


def struct_docs(members, forms):
    """P7's variants of one base value: members [(name, text)], and each field's forms."""
    fields = [k for k, _ in members]

    def with_(extra):
        return obj(members + list(extra))

    out = [obj(members), obj(list(reversed(members)))]
    ws = b" {"
    for k, (key, val) in enumerate(members):
        if k > 0:
            ws += b","
        ws += b'\n\t"' + key.encode() + b'" : ' + val + b" "
    ws += b"\r\n} "
    out.append(ws)
    out.append(obj([("zz", b"1")] + members))

    def deep(d):
        return b"[" * d + b"]" * d

    unknown = [b'{"a":[1,2.5,{"b":null}],"c":"d"}', b'{"a":1,"a":2}', b'{"a":1,"\\u0061":2}', deep(100), deep(128),
               deep(129), deep(130), b'"a\x01b"', b'"a\xffb"', b'"\\ud800"', b'"\\udc00"', b'{"\\ud800":1}', b'"\\x"',
               b"1e400", b"18446744073709551616", b"-0", b"01", b"[1,]", b"nul"]
    for val in unknown:
        out.append(with_([("zz", val)]))
    out.append(with_([("zz", b"1"), ("zz", b"2")]))
    out.append(with_([("zz", b"1"), ("\\u007a\\u007a", b"2")]))
    bk = obj(members + [("z\ufffd", b"1")])
    at = bk.index("\ufffd".encode())
    out.append(bk[:at] + b"\xff" + bk[at + 3:])
    out.append(with_([("\\udc00", b"1")]))
    k0 = members[0][0]
    out.append(obj([("\\u%04x%s" % (k0.encode()[0], k0[1:]), members[0][1])] + members[1:]))
    out.append(obj([(members[0][0].upper(), members[0][1])] + members[1:]))
    out.append(obj([(members[0][0] + " ", members[0][1])] + members[1:]))
    for j, field in enumerate(fields):
        out.append(obj(members[:j] + members[j + 1:]))
        out.append(with_([(field, members[j][1])]))
        out.append(obj(members[:j] + [(field, b"null")] + members[j + 1:]))
        for form in forms[j]:
            out.append(obj(members[:j] + [(field, form)] + members[j + 1:]))
    vals = [val for _, val in members]
    out.append(wrap("[", vals, "]"))
    out.append(wrap(" [ ", vals, " ] "))
    out.append(wrap("[", vals[:-1], "]"))
    out.append(wrap("[", vals + [b"0"], "]"))
    out.append(wrap("[", vals, ",]"))
    out.append(b"[]")
    doc = obj(members)
    n = len(doc)
    out.append(doc[:n - 1] + b",}")
    out.append(doc[:n - 1])
    out.append(doc[:n // 2])
    out.append(doc + b" x")
    out.append(doc + doc)
    out.append(b"\xef\xbb\xbf" + doc)
    c = doc.index(b":")
    out.append(doc[:c] + doc[c + 1:])
    out.append(b"{1:2}")
    out.append(b"{}")
    return out


def shapes_value(x):
    return ("obj", [("x", ("f", x["x"])), ("n", ("u", x["n"])), ("k", ("u", x["k"])), ("flag", ("b", x["flag"])),
                    ("name", ("s", x["name"])), ("xs", fl(x["xs"])), ("ns", ("list", [("u", w) for w in x["ns"]])),
                    ("labels", ("list", [("s", t) for t in x["labels"]])),
                    ("m", ("list", [fl(r) for r in x["m"]])), ("kernel", ("unit", x["kernel"]))])


def shapes_toks(x):
    t = [f(x["x"]), i(x["n"]), i(x["k"]), b(x["flag"]), s(x["name"]), v(len(x["xs"]))] + [f(y) for y in x["xs"]]
    t += [v(len(x["ns"]))] + [i(w) for w in x["ns"]]
    t += [v(len(x["labels"]))] + [s(lb) for lb in x["labels"]]
    t += [v(len(x["m"]))]
    for r in x["m"]:
        t += [v(len(r))] + [f(y) for y in r]
    t.append(e("Kernel", x["kernel"]))
    return t


def random_shapes(g):
    n = g.int(0, 4)
    x = {"x": finite(g), "n": g.word()}
    w = g.word()
    x["k"] = w >> g.int(0, 63)
    x["flag"] = g.int(0, 1) == 1
    x["name"] = random_string(g)
    x["xs"] = [finite(g) for _ in range(g.int(0, 6))]
    x["ns"] = [g.word() for _ in range(g.int(0, 6))]
    x["labels"] = [random_string(g) for _ in range(g.int(0, 4))]
    x["m"] = [[finite(g) for _ in range(g.int(0, 3))] for _ in range(n)]
    x["kernel"] = KERNELS[g.int(0, 3)]
    return x


def any_f64(g):
    k = g.int(0, 15)
    return NAN if k == 0 else (INF if k == 1 else finite(g))


def rect(g, rows, cols):
    return [[any_f64(g) for _ in range(cols)] for _ in range(rows)]


def mat_toks(rows):
    out = [v(len(rows))]
    for r in rows:
        out += [v(len(r))] + [f(x) for x in r]
    return out


def structs(m, g):
    pair = [("mean", b"1.5"), ("std_dev", b"0.25")]
    docs = struct_docs(pair, [NUMBERS, NUMBERS])
    for k in range(150):
        mu = finite(g)
        sd = finite(g)
        docs.append(ser(("obj", [("mean", ("f", mu)), ("std_dev", ("f", sd))]), k % 5 == 0).encode())
    for d in docs:
        read(m, "json::Pair", d)
    ints = B("0", "1", "-1", "-0", "1.0", "1e3", "9223372036854775808", "18446744073709551615", "18446744073709551616",
             "\"1\"", "true", "[]")
    bools = B("true", "false", "1", "\"true\"", "True")
    texts = STRINGS + B("1", "[]", "{}")
    vf = B("[]", "[1,2]", "[null]", "[\"1\"]", "[[1]]", "1", "{}", "[-0]", "[1e400]", "[1,]", "[18446744073709551615]")
    vu = B("[]", "[1]", "[-1]", "[1.0]", "[18446744073709551615]", "[18446744073709551616]", "[\"1\"]", "1")
    vs = B("[]", "[\"a\"]", "[1]", "[null]", '["\\ud800"]', "\"a\"", "[\"a\",[]]")
    vm = B("[]", "[[]]", "[[1],[2,3]]", "[1]", "[[null]]", "[[1e400]]", "[[-0]]", "[[1],[]]", "{}") + [b"[" * 129 + b"]" * 129]
    kern = B("\"Gaussian\"", "\"Triangular\"", "{\"Uniform\":null}", "\"gaussian\"", "\"Cosine\"", "0",
             "{\"Gaussian\":0}", "{}", "[\"Gaussian\"]", "null")
    base = {"x": 0.5, "n": 7, "k": 3, "flag": True, "name": "t-test", "xs": [1.0, -0.0, 2.5], "ns": [0, 1 << 63],
            "labels": ["a", "é"], "m": [[1.0, 2.0], [3.0]], "kernel": "Epanechnikov"}
    members = [(key, ser(val).encode()) for key, val in shapes_value(base)[1]]
    docs = struct_docs(members, [NUMBERS, ints, ints, bools, texts, vf, vu, vs, vm, kern])
    for k in range(40):
        docs.append(ser(shapes_value(random_shapes(g)), k % 4 == 0).encode())
    for d in docs:
        read(m, "json::Shapes", d)


def enums(m):
    docs = B("\"Gaussian\"", "\"Epanechnikov\"", "\"Uniform\"", "\"Triangular\"", " \"Uniform\" ", "{\"Gaussian\":null}",
             "{\"Epanechnikov\":null}", "{\"Uniform\":null}", "{\"Triangular\":null}", "{ \"Triangular\" : null }",
             "\"gaussian\"", "\"GAUSSIAN\"", "\"Cosine\"", "\"\"", "\"\\u0047aussian\"", "{\"\\u0047aussian\":null}",
             "{\"Gaussian\":[]}", "{\"Gaussian\":0}", "{\"Gaussian\":{}}", "{\"Gaussian\":nul}", "{\"Gaussian\" null}",
             "{\"Gaussian\":null,}", "{\"Gaussian\":null,\"Uniform\":null}", "{\"Gaussian\":null,\"Gaussian\":null}",
             "{}", "[\"Gaussian\"]", "0", "null", "\"Gaussian", "\"Gaussian\" x", "{\"Gaussian\":null} ") + COMMON
    for d in docs:
        read(m, "json::Kernel", d)
    docs = B("{\"InvalidParameter\":\"m\"}", "{\"InvalidSample\":\"s\"}", "{\"ConvergenceFailure\":\"c\"}",
             "{\"DimensionMismatch\":\"d\"}", "{\"ComputationError\":\"e\"}", " { \"InvalidParameter\" :\n\"m\" } ",
             "\"InvalidParameter\"", "{\"InvalidParameter\":null}", "{\"InvalidParameter\":5}",
             "{\"InvalidParameter\":[\"m\"]}", "{\"Nope\":\"m\"}", "{\"InvalidParameter\":\"m\",\"InvalidSample\":\"n\"}",
             "{\"invalidParameter\":\"m\"}", "{\"\\u0049nvalidParameter\":\"m\"}", "{\"InvalidParameter\":\"m\",}", "{}",
             "[]", "[\"m\"]")
    docs += [b'{"InvalidParameter":' + t + b"}" for t in STRINGS] + COMMON
    for d in docs:
        read(m, "json::PramanaError::code", d, "PramanaError", code_only=True)
    for d in docs:
        read(m, "json::PramanaError", d)


def writes(m, g):
    xs = [0.0, -0.0, 1.5, NAN, INF, -INF, MAXF, 5e-324] + [finite(g) for _ in range(200)]
    for x in xs:
        write(m, "f64", [f(x)])
    ws = [0, 1, 9, 10, 99, 100, (1 << 53) + 1, (1 << 63) - 1, 1 << 63, U64] + [g.word() >> (k % 64) for k in range(200)]
    for w in ws:
        write(m, "u64", [i(w)])
    for x in (False, True):
        write(m, "bool", [b(x)])
    texts = [""] + ["a%sb" % chr(c) for c in range(0x20)]
    texts += ["\"", "\\", "/", "\x7f", "é", "😀", "\u2028", "\uffff", "\U0010ffff", "%"]
    texts += [random_string(g) for _ in range(300)]
    for t in texts:
        write(m, "String", [s(t)])
    for _ in range(150):
        x = [any_f64(g) for _ in range(g.int(0, 10))]
        write(m, "Vec<f64>", [v(len(x))] + [f(y) for y in x])
    for _ in range(100):
        n = g.int(0, 10)
        x = [g.word() >> (k * 7 % 64) for k in range(n)]
        write(m, "Vec<u64>", [v(len(x))] + [i(w) for w in x])
    for _ in range(100):
        x = [random_string(g) for _ in range(g.int(0, 5))]
        write(m, "Vec<String>", [v(len(x))] + [s(t) for t in x])
    for k in range(100):
        rows = g.int(0, 4)
        cols = 0 if k % 10 == 0 else g.int(1, 4)
        write(m, "Vec<Vec<f64>>", mat_toks(rect(g, rows, cols)))
    for _ in range(100):
        mu = any_f64(g)
        write(m, "Pair", [f(mu), f(any_f64(g))])
    for _ in range(60):
        x = random_shapes(g)
        rows = len(x["m"])
        cols = g.int(0, 3)
        x["m"] = rect(g, rows, cols)
        write(m, "Shapes", shapes_toks(x))
    for k in KERNELS:
        write(m, "Kernel", [e("Kernel", k)])
    for name in ["InvalidParameter", "InvalidSample", "ConvergenceFailure", "DimensionMismatch", "ComputationError"]:
        write(m, "PramanaError", [e("PramanaError", name)])


def simple_rng(m, g):
    ints = B("0", "1", "-1", "-0", "1.0", "1e3", "9223372036854775807", "9223372036854775808", "18446744073709551615",
             "18446744073709551616", "\"1\"", "true", "[]", "[1]")
    docs = struct_docs([("state", b"42")], [ints])
    seeds = [0, 1, 1 << 63, U64] + [g.word() >> (k % 64) for k in range(60)]
    docs += [b'{"state":%d}' % w for w in seeds]
    for d in docs:
        read(m, "json::SimpleRng", d)
    for w in seeds:
        write(m, "SimpleRng", [i(w)])


F64_FORMS = B("0", "-0", "1", "-1", "2", "1.5", "-3.5", "1e300", "-1e300", "1e-300", "5e-324", "1.7976931348623157e308",
              "1e309", "-1e309", "1e999", "null", "\"1\"", "true", "[]", "{}", "1.0e0", "0.1")


def jt(x):
    return V.json_f64_text(x)


def normal(m, g):
    docs = struct_docs([("mean", jt(1.5).encode()), ("std_dev", jt(2.3).encode())], [F64_FORMS, F64_FORMS])
    docs += B("[0.5,2.0]", "[0.5]", "[0.5,2.0,1]", "[0.5,-2.0]")
    pairs = [(0.0, 1.0), (-0.0, 1.0), (1e-300, 1e300), (MAXF, MIN_POSITIVE)]
    for _ in range(40):
        mu = g.signed_log_uniform(-20.0, 20.0)
        pairs.append((mu, g.log_uniform(-20.0, 20.0)))
    for mu, sd in pairs:
        docs.append(('{"mean":%s,"std_dev":%s}' % (jt(mu), jt(sd))).encode())
        docs.append(('{"mean":%s,"std_dev":%s}' % (jt(mu), jt(-sd))).encode())
    for d in docs:
        read(m, "json::Normal", d)
    for mu, sd in pairs:
        write(m, "Normal", [f(mu), f(sd)])


def uniform(m, g):
    docs = struct_docs([("min", jt(-1.0).encode()), ("max", jt(5.0).encode())], [F64_FORMS, F64_FORMS])
    docs += B("[0.5,2.0]", "[0.5]", "[2.0,0.5]", "[0.5,2.0,1]", '{"min":-1e308,"max":1e308}')
    pairs = [(0.0, 1.0), (-0.0, 1.0), (-MAXF, 0.0), (MIN_POSITIVE, 1e300)]
    for _ in range(40):
        lo = g.signed_log_uniform(-20.0, 20.0)
        pairs.append((lo, lo + g.log_uniform(-20.0, 20.0)))
    pairs = [p for p in pairs if V.d_uniform_new(*p)]
    for lo, hi in pairs:
        docs.append(('{"min":%s,"max":%s}' % (jt(lo), jt(hi))).encode())
        docs.append(('{"min":%s,"max":%s}' % (jt(hi), jt(lo))).encode())
    for d in docs:
        read(m, "json::Uniform", d)
    for lo, hi in pairs:
        write(m, "Uniform", [f(lo), f(hi)])


def one_field(m, g, name, field, base, extra, fixed, sign_doc, count=40):
    forms = F64_FORMS + extra[0]
    docs = struct_docs([(field, jt(base).encode())], [forms])
    docs += extra[1]
    vals = list(fixed)
    vals += [g.log_uniform(-20.0, 20.0) for _ in range(count)] if sign_doc != "p" else [g.unit() for _ in range(count)]
    for x in vals:
        docs.append(('{"%s":%s}' % (field, jt(x))).encode())
        other = x + 1.0 if sign_doc == "p" else -x
        docs.append(('{"%s":%s}' % (field, jt(other))).encode())
    for d in docs:
        read(m, f"json::{name}", d)
    for x in vals:
        write(m, name, [f(x)])


def two_field(m, g, name, fields, base, extra, fixed, negate_second, gen):
    docs = struct_docs([(fields[0], jt(base[0]).encode()), (fields[1], jt(base[1]).encode())], [F64_FORMS, F64_FORMS])
    docs += extra
    pairs = list(fixed)
    for _ in range(40):
        pairs.append(gen())
    for a, c in pairs:
        docs.append(('{"%s":%s,"%s":%s}' % (fields[0], jt(a), fields[1], jt(c))).encode())
        if negate_second:
            docs.append(('{"%s":%s,"%s":%s}' % (fields[0], jt(a), fields[1], jt(-c))).encode())
        else:
            docs.append(('{"%s":%s,"%s":%s}' % (fields[0], jt(-a), fields[1], jt(c))).encode())
    for d in docs:
        read(m, f"json::{name}", d)
    for a, c in pairs:
        write(m, name, [f(a), f(c)])


def binomial(m, g):
    u64_forms = B("0", "1", "10", "-1", "1.0", "1e3", "9223372036854775807", "9223372036854775808",
                  "18446744073709551615", "18446744073709551616", "\"10\"", "true", "null", "[]")
    forms = F64_FORMS + B("0.9999999999999999", "1.0000000000000002", "0.5")
    docs = struct_docs([("n", b"10"), ("p", jt(0.3).encode())], [u64_forms, forms])
    docs += B("[10,0.3]", "[10]", "[0,0.3]", "[10,1.5]", "[10,0.3,1]", '{"n":0,"p":0.5}', '{"n":5,"p":1.5}')
    pairs = [(1, 0.0), (U64, 1.0), (1 << 63, 0.5)]
    for k in range(40):
        n = g.word() >> (k % 64)
        pairs.append((n, g.unit()))
    pairs = [p for p in pairs if p[0] != 0]
    for n, p in pairs:
        docs.append(('{"n":%d,"p":%s}' % (n, jt(p))).encode())
        docs.append(('{"n":%d,"p":%s}' % (n, jt(p + 1.0))).encode())
    for d in docs:
        read(m, "json::Binomial", d)
    for n, p in pairs:
        write(m, "Binomial", [i(n), f(p)])


def multivariate_normal(m, g):
    means = B("[1.0,2.0]", "[1,2]", "[]", "[1.5]", "[1,2,3]", "[null,2]", "[1e309,2]", "[\"1\",2]", "null", "{}", "1",
              "[[1,2]]", "[1,2,]", "[-0.0,0]", "[1.7976931348623157e308,5e-324]")
    covs = B("[[4,1],[1,9]]", "[]", "[[]]", "[[4,1]]", "[[4,1],[1]]", "[[4,1],[2,9]]", "[[1,2],[2,1]]",
             "[[4,1],[1,9],[0,0]]", "[[4,1,0],[1,9,0]]", "[[4,null],[1,9]]", "[[4,1e309],[1,9]]", "[4,1,1,9]", "null",
             "[[4,\"1\"],[1,9]]", "[[4.000000000000001,1],[1,9]]", "[[4,1],[1.0000000000000002,9]]", "{}",
             "[[1,0],[0,1]]", "[[0,0],[0,0]]", "[[-4,1],[1,9]]", "[[4,1],[1,-0]]", "[[4,-0.0],[-0.0,9]]")
    docs = struct_docs([("mean", b"[1.0,2.0]"), ("covariance", b"[[4.0,1.0],[1.0,9.0]]")], [means, covs])
    docs += B('{"mean":[0,0,0],"covariance":[[1,0,0],[0,1,0],[0,0,1]],"cholesky_l":[[1]],"log_det":5.0,"dim":1}',
              '{"mean":[1,2],"covariance":[[4,1],[1,9]],"cholesky_l":"junk","log_det":null,"dim":-1}',
              '{"mean":[1,2],"covariance":[[4,1],[1,9]],"cholesky_l":[[2,0],[0.5,3]],"log_det":1e309,"dim":2}',
              '{"mean":[0,0],"covariance":[[1,2],[2,1]]}', '{"mean":[],"covariance":[]}', '{"mean":[0],"covariance":[[0]]}',
              '{"mean":[0],"covariance":[[5e-324]]}',
              '{"mean":[0,0,0],"covariance":[[5e-324,0,1e200],[0,1,0],[1e200,0,1]]}',
              '{"mean":[0,0],"covariance":[[4,1],[1.0000000001,9]]}', '{"mean":[0,0],"covariance":[[4,1],[1.00000000001,9]]}',
              '{"covariance":[[4,1],[1,9]],"mean":[1,2]}', "[[1,2],[[4,1],[1,9]]]", "[[1,2]]",
              "[[1,2],[[4,1],[1,9]],5]", "[]")
    cases = mvn_cases(g)
    docs += [V.serde_write("MultivariateNormal", iter(parsed(mvn_toks(c)))) for c in cases]
    for d in docs:
        read(m, "json::MultivariateNormal", d)
    for c in cases:
        write(m, "MultivariateNormal", mvn_toks(c))


def emit():
    m = Module("json")
    g = Rng(0xBB67AE8584CAA73B)
    fls = floats()
    for d in COMMON + NUMBERS + fls:
        read(m, "json::f64", d)
    words = [str(g.word() >> (k % 64)).encode() for k in range(200)]
    for d in COMMON + NUMBERS + words:
        read(m, "json::u64", d)
    for d in COMMON + NUMBERS:
        read(m, "json::usize", d)
    bools = B("true", "false", " true ", "True", "TRUE", "tru", "truee", "true false", "\"true\"", "t")
    for d in COMMON + bools:
        read(m, "json::bool", d)
    texts = [V.serde_escape(random_string(g).encode("utf-8")) for _ in range(300)]
    texts += [escaped_string(g) for _ in range(200)]
    for d in COMMON + STRINGS + texts:
        read(m, "json::String", d)
    vecs = B("[ ]", "[1,2,3]", " [ 1 , 2 ] ", "[1,]", "[,]", "[,1]", "[1 2]", "[1,,2]", "[1", "[null]", "[1,null]",
             "[\"1\"]", "[true]", "[{}]", "[[]]")
    for n in NUMBERS:
        vecs += [wrap("[", [n], "]"), wrap("[", [b"1", n], "]")]
    for k in range(150):
        xs = [finite(g) for _ in range(g.int(0, 12))]
        vecs.append(ser(fl(xs), k % 5 == 0).encode())
    for d in COMMON + vecs:
        read(m, "json::Vec<f64>", d)
    uvecs = B("[ ]", "[0,1,2]", "[1,]", "[1.0]", "[-0]", "[-1]", "[null]", "[\"1\"]", "[[1]]")
    uvecs += [wrap("[", [n], "]") for n in NUMBERS]
    for _ in range(100):
        n = g.int(0, 10)
        xs = [g.word() >> (k * 7 % 64) for k in range(n)]
        uvecs.append(ser(("list", [("u", w) for w in xs])).encode())
    for d in COMMON + uvecs:
        read(m, "json::Vec<u64>", d)
    svecs = B("[ ]", "[\"a\",\"b\"]", "[\"a\",1]", "[null]", "[\"a\",]", "[[\"a\"]]")
    for t in STRINGS:
        svecs += [wrap("[", [t], "]"), wrap("[", [b'"ok"', t], "]")]
    for _ in range(100):
        xs = [random_string(g) for _ in range(g.int(0, 6))]
        svecs.append(ser(("list", [("s", t) for t in xs])).encode())
    for d in COMMON + svecs:
        read(m, "json::Vec<String>", d)
    mats = B("[ ]", "[[]]", "[[],[]]", "[[1],[2,3]]", "[[1,2],[3,4]]", "[1]", "[[1],1]", "[[1],null]", "[null]",
             "[[\"x\"]]", "[[[1]]]", "[[1e400]]", "[[-0]]", "[[18446744073709551615]]", "[[1],]", "[[1,]]",
             " [ [ 1 , 2 ] , [ 3 ] ] ")
    mats += [b"[" * d + b"]" * d for d in (3, 100, 128, 129, 200)]
    for k in range(100):
        rows = g.int(0, 5)
        ragged = k % 4 == 0
        cols = g.int(0, 5)
        mat = []
        for _ in range(rows):
            c = g.int(0, 5) if ragged else cols
            mat.append([finite(g) for _ in range(c)])
        mats.append(ser(("list", [fl(r) for r in mat])).encode())
    for d in COMMON + mats:
        read(m, "json::Vec<Vec<f64>>", d)
    structs(m, g)
    enums(m)
    writes(m, g)
    simple_rng(m, g)
    normal(m, g)
    uniform(m, g)
    one_field(m, g, "Exponential", "lambda", 2.5, ([], B("[2.5]", "[]", "[2.5,1]", "[-2.5]")),
              [1.0, MIN_POSITIVE, from_bits(1), MAXF], "neg")
    one_field(m, g, "Bernoulli", "p", 0.7,
              (B("0.9999999999999999", "1.0000000000000002", "1.00000000000000001", "-5e-324", "0.5"),
               B("[0.7]", "[]", "[0.7,1]", "[1.5]")), [0.0, -0.0, 1.0, 0.5, 1.0 - EPS / 2.0, from_bits(1)], "p")
    two_field(m, g, "Cauchy", ["x0", "gamma"], (2.0, 3.0),
              B("[0.5,2.0]", "[0.5]", "[0.5,-2.0]", "[0.5,2.0,1]", '{"x0":0.0,"gamma":0.0}'),
              [(0.0, 1.0), (-0.0, 1.0), (MAXF, from_bits(1)), (-1e-300, 1e300)], True,
              lambda: (g.signed_log_uniform(-20.0, 20.0), g.log_uniform(-20.0, 20.0)))
    two_field(m, g, "Weibull", ["k", "lambda"], (2.0, 3.0),
              B("[0.5,2.0]", "[0.5]", "[-0.5,2.0]", "[0.5,2.0,1]", '{"k":-1.0,"lambda":1.0}'),
              [(1.0, 1.0), (MAXF, from_bits(1)), (1e-300, 1e300)], False,
              lambda: (g.log_uniform(-20.0, 20.0), g.log_uniform(-20.0, 20.0)))
    two_field(m, g, "Gamma", ["alpha", "beta"], (2.0, 3.0),
              B("[0.5,2.0]", "[0.5]", "[0.5,-2.0]", "[0.5,2.0,1]", '{"alpha":0.0,"beta":1.0}'),
              [(1.0, 1.0), (MAXF, from_bits(1)), (1e-300, 1e300)], True,
              lambda: (g.log_uniform(-20.0, 20.0), g.log_uniform(-20.0, 20.0)))
    one_field(m, g, "ChiSquared", "df", 3.0, ([], B("[3.0]", "[]", "[3.0,1]", "[-3.0]", '{"df":0.0}')),
              [1.0, MIN_POSITIVE, from_bits(1), MAXF], "neg")
    for name, fields in (("Beta", ["alpha", "beta"]), ("FDistribution", ["d1", "d2"])):
        zero = '{"%s":1.0,"%s":0.0}' % (fields[0], fields[1])
        two_field(m, g, name, fields, (2.0, 3.0), B("[0.5,2.0]", "[0.5]", "[0.5,-2.0]", "[0.5,2.0,1]", zero),
                  [(1.0, 1.0), (MAXF, from_bits(1)), (1e-300, 1e300)], True,
                  lambda: (g.log_uniform(-20.0, 20.0), g.log_uniform(-20.0, 20.0)))
        if name == "Beta":
            one_field(m, g, "StudentT", "df", 3.0, ([], B("[3.0]", "[]", "[3.0,1]", "[-3.0]", '{"df":0.0}')),
                      [1.0, MIN_POSITIVE, from_bits(1), MAXF], "neg")
    one_field(m, g, "Poisson", "lambda", 3.5, ([], B("[3.5]", "[]", "[3.5,1]", "[-3.5]", '{"lambda":-3.0}')),
              [1.0, 30.0, from_bits(1), MAXF], "neg")
    binomial(m, g)
    multivariate_normal(m, g)
    kernel_density(m, Rng(0x3C6EF372FE94F82B))
    pca_result(m, Rng(0xA54FF53A5F1D36F1))
    test_results(m, Rng(0x13198A2E03707344))
    regression_models(m, Rng(0x299F31D0082EFA98))
    arima_models(m, Rng(0xEC4E6C89452821E6))
    bayesian_update(m, Rng(0x5F6F7E8D9CABBACD))
    return m

def kde_toks(data, h, k):
    return [v(len(data))] + [f(x) for x in data] + [f(h), e("Kernel", k)]


def kernel_density(m, g):
    """M4 part a2: KernelDensity's codec through `KernelDensity::new` (serde's try_from)."""
    datas = B("[1.0,2.0,3.0]", "[1,2,3]", "[]", "[5]", "[1,null]", "[1e309]", "[\"1\"]", "null", "{}", "1", "[[1]]",
              "[1,2,]", "[-0.0,0]", "[1.7976931348623157e308,5e-324]")
    hs = B("0.5", "0", "-0.0", "-1", "1e309", "5e-324", "1.7976931348623157e308", "null", "\"0.5\"", "[0.5]", "true")
    kernels = B('"Gaussian"', '"Epanechnikov"', '"Uniform"', '"Triangular"', '"gaussian"', '"Box"', "0", '{"Uniform":null}',
                '{"Uniform":1}', "null", '["Gaussian"]', '""')
    docs = struct_docs([("data", b"[1.0,2.0,3.0]"), ("bandwidth", b"0.5"), ("kernel", b'"Gaussian"')],
                       [datas, hs, kernels])
    docs += B('{"data":[1,2],"bandwidth":0.5,"kernel":"Gaussian","extra":[1]}', "[[1,2],0.5,\"Triangular\"]",
              "[[1,2],0.5]", "[[1,2],0.5,\"Uniform\",1]", '{"data":[1,2],"bandwidth":0.5}', "[]")
    cases = [([1.0, 2.0, 3.0], 0.5, "Gaussian"), ([0.0], 1.0, "Epanechnikov"), ([-0.0, 0.0], 5e-324, "Uniform"),
             ([MAXF, -MAXF], MAXF, "Triangular")]
    for k in range(20):
        n = g.int(1, 9)
        cases.append(([finite(g) for _ in range(n)], g.log_uniform(-30.0, 30.0), KERNELS[g.int(0, 3)]))
    docs += [V.serde_write("KernelDensity", iter(parsed(kde_toks(*c)))) for c in cases]
    for d in docs:
        read(m, "json::KernelDensity", d)
    for c in cases:
        write(m, "KernelDensity", kde_toks(*c))


def pca_toks(comps, ev, ra):
    out = [v(len(comps))]
    for r in comps:
        out += [v(len(r))] + [f(x) for x in r]
    return out + [v(len(ev))] + [f(x) for x in ev] + [v(len(ra))] + [f(x) for x in ra]


def pca_result(m, g):
    """M4 part a3: PcaResult's derived codec (no validation: any shapes, ragged rows among them)."""
    comps = B("[[1.0,0.0],[0.0,1.0]]", "[]", "[[]]", "[[1],[2,3]]", "[[1,null]]", "[1,2]", "null", "[[1e309]]", "{}",
              "[[\"1\"]]", "[[1,2],]")
    evs = B("[2.0,1.0]", "[]", "[1,null]", "null", "[[1]]", "[1e309]", "1")
    docs = struct_docs([("components", b"[[1.0,0.0],[0.0,1.0]]"), ("eigenvalues", b"[2.0,1.0]"),
                        ("explained_variance_ratio", b"[0.667,0.333]")], [comps, evs, evs])
    docs += B('{"components":[[1,2]],"eigenvalues":[1],"explained_variance_ratio":[1],"x":1}', "[[[1]],[2],[3]]",
              "[[[1]],[2]]", '{"components":[[1]],"eigenvalues":[1]}', "[]")
    cases = [([[1.0, 0.0], [0.0, 1.0]], [2.0, 1.0], [0.667, 0.333]), ([], [], []), ([[1.0], [2.0, 3.0], []], [5e-324],
             [MAXF, -0.0])]
    for k in range(12):
        p = g.int(1, 4)
        cases.append(([[finite(g) for _ in range(p)] for _ in range(p)], [finite(g) for _ in range(p)],
                      [g.unit() for _ in range(p)]))
    docs += [V.serde_write("PcaResult", iter(parsed(pca_toks(*c)))) for c in cases]
    for d in docs:
        read(m, "json::PcaResult", d)
    for c in cases:
        write(m, "PcaResult", pca_toks(*c))


def test_results(m, g):
    """M4 part b1: TestResult, AnovaResult and KsResult, plain derives (no validation)."""
    names = B('"one-sample t-test"', '""', '"a\\"b\\\\c"', '"\\u00e9t\\u00e9"', '"\\n\\t\\u0001"', "null", "1",
              '["x"]', '"\\ud800"')
    nums = B("1.5", "0", "-0.0", "1e309", "null", '"1"', "true")
    bools = B("false", "true", "0", "null", '"true"')
    docs = struct_docs([("test_name", b'"one-sample t-test"'), ("statistic", b"1.5"), ("p_value", b"0.2"),
                        ("degrees_of_freedom", b"9.0"), ("reject_at_alpha", b"0.05"), ("reject", b"false")],
                       [names, nums, nums, nums, nums, bools])
    for d in docs:
        read(m, "json::TestResult", d)
    usz = B("1", "0", "-1", "1.0", "18446744073709551615", "18446744073709551616", "null")
    adocs = struct_docs([("ss_between", b"1.0"), ("ss_within", b"2.0"), ("ss_total", b"3.0"), ("df_between", b"1"),
                         ("df_within", b"8"), ("ms_between", b"1.0"), ("ms_within", b"0.25"), ("f_statistic", b"4.0"),
                         ("p_value", b"0.07"), ("reject_at_alpha", b"0.05"), ("reject", b"false")],
                        [nums, nums, nums, usz, usz, nums, nums, nums, nums, nums, bools])
    for d in adocs:
        read(m, "json::AnovaResult", d)
    kdocs = struct_docs([("statistic", b"0.25"), ("p_value", b"0.5"), ("reject_at_alpha", b"0.05"), ("reject", b"true")],
                        [nums, nums, nums, bools])
    for d in kdocs:
        read(m, "json::KsResult", d)
    for k in range(12):
        name = ["one-sample t-test", "two-sample Welch t-test", "chi-squared test", "", "a\"b\\c\n",
                "\u00e9t\u00e9"][k % 6]
        write(m, "TestResult", [("s", name.encode("utf-8")), f(finite(g)), f(g.unit()), f(finite(g)), f(g.unit()),
                                i(k % 2)])
        write(m, "AnovaResult", [f(finite(g)) for _ in range(3)] + [i(g.int(0, 100)), i(g.word())]
              + [f(finite(g)) for _ in range(5)] + [i(k % 2)])
        write(m, "KsResult", [f(g.unit()), f(g.unit()), f(g.unit()), i(k % 2)])
        write(m, "ConfidenceInterval", [f(finite(g)), f(finite(g)), f(finite(g)), f(g.unit())])
    cdocs = struct_docs([("lower", b"4.8"), ("upper", b"5.2"), ("estimate", b"5.0"), ("confidence_level", b"0.95")],
                        [nums, nums, nums, nums])
    for d in cdocs:
        read(m, "json::ConfidenceInterval", d)


def regression_models(m, g):
    """M5 part a: LinearModel, PolynomialModel and LogisticModel, plain derives (no validation)."""
    nums = B("1.5", "0", "-0.0", "1e309", "null", '"1"', "true")
    vecs = B("[1.0,2.0]", "[]", "[1,null]", "null", "[[1]]", "[1e309]", "1", "[1,]")
    usz = B("1", "0", "-1", "1.0", "18446744073709551615", "18446744073709551616", "null")
    bools = B("false", "true", "0", "null")
    for d in struct_docs([("slope", b"2.0"), ("intercept", b"1.0"), ("r_squared", b"0.99")], [nums, nums, nums]):
        read(m, "json::LinearModel", d)
    for d in struct_docs([("coefficients", b"[1.0,2.0]"), ("degree", b"1"), ("r_squared", b"0.99")], [vecs, usz, nums]):
        read(m, "json::PolynomialModel", d)
    for d in struct_docs([("coefficients", b"[0.5,1.2]"), ("iterations", b"7"), ("converged", b"true")],
                         [vecs, usz, bools]):
        read(m, "json::LogisticModel", d)
    for k in range(12):
        write(m, "LinearModel", [f(finite(g)), f(finite(g)), f(g.unit())])
        cs = [finite(g) for _ in range(k % 5)]
        write(m, "PolynomialModel", [v(len(cs))] + [f(c) for c in cs] + [i(g.int(0, 9)), f(g.unit())])
        write(m, "LogisticModel", [v(len(cs))] + [f(c) for c in cs] + [i(g.int(0, 100)), i(k % 2)])


def arima_models(m, g):
    """M5 part b: ArimaModel, a plain derive (no validation)."""
    nums = B("1.5", "0", "-0.0", "1e309", "null", '"1"', "true")
    vecs = B("[0.6,-0.2]", "[]", "[1,null]", "null", "[[1]]", "[1e309]", "1")
    usz = B("1", "0", "-1", "1.0", "18446744073709551615", "18446744073709551616", "null")
    for d in struct_docs([("ar_coefficients", b"[0.6,-0.2]"), ("d", b"1"), ("intercept", b"0.25"),
                          ("residual_variance", b"1.5")], [vecs, usz, nums, nums]):
        read(m, "json::ArimaModel", d)
    for k in range(12):
        cs = [finite(g) for _ in range(k % 4)]
        write(m, "ArimaModel", [v(len(cs))] + [f(c) for c in cs] + [i(g.int(0, 3)), f(finite(g)), f(g.unit())])




def bayesian_update(m, g):
    """M5 part c: BayesianUpdate's codec through `BayesianUpdate::new` (serde's try_from)."""
    nums = B("0.5", "0", "-0.0", "1e309", "null", '"1"', "true", "1.5", "-0.1")
    docs = struct_docs([("prior", b"0.3"), ("likelihood", b"0.7"), ("evidence", b"0.4")], [nums, nums, nums])
    docs += B('{"prior":0.5,"likelihood":0.5,"evidence":0.0}', '{"prior":1.5,"likelihood":0.5,"evidence":0.9}',
              '{"prior":0.3,"likelihood":0.8,"evidence":0.5}', '{"prior":0.5,"likelihood":0.9,"evidence":0.2}')
    for d in docs:
        read(m, "json::BayesianUpdate", d)
    for _ in range(12):
        p, l = g.unit(), g.unit()
        write(m, "BayesianUpdate", [f(p), f(l), f(min(1.0, p * l + 0.25))])

