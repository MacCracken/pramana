"""`ai.tsv` (M7 part b): 1.3.0's ai.rs over hoosh 1.3.0's client. Rust's `{:.p}` (exact, ties to even: the dyadic
ties k/2^(p+1) among them) and `{}` (the shortest digits, positional) of seeded and special doubles; the data summary
(columns in name order, ADR 007 row 9; empty, NaN-bearing and constant columns); the request body serde_json writes
(`json!`'s sorted keys, its escapes); `StatQuery::query` over a recorded reply: the URL and body it posts, then the
direct answer, or the refusal for a non-2xx status or a reply hoosh's `ChatCompletionResp` refuses (Option fields
absent or null, u32 usage bounds, unknown members, the sequence form); `execute_tool`, each of the five tools and every
refusal (1.3.0's bounded-argument tests among them). Class C: the t-test rows (the T1 incomplete beta); the rest is
arithmetic and text."""
import math
import urllib.parse

from .grids import INF, MAXF, NAN, TINY, Rng
from .models import V
from .tsv import Module, f, from_bits, from_model, i, parsed, s, v


def fl(xs):
    return [v(len(xs))] + [f(x) for x in xs]


def cols_toks(cols):
    out = []
    for name, xs in cols:
        out += [s(name)] + fl(xs)
    return out


def row(m, fn, cls, ins):
    sl, sc = V.take_slices(parsed(ins))
    m.row(fn, cls, ins, from_model(V.AI[fn](_prims(), sl, sc)))


_P = None


def _prims():
    global _P
    if _P is None:
        from pathlib import Path
        _P = V.dist_prims(Path(__file__).resolve().parents[3])[0]
    return _P


def series(n, a=0.4, b=0.05):
    return [math.sin(k * a) + b * k for k in range(n)]


def emit():
    m = Module("ai")
    g = Rng(0x9B05688C2B3E6C1F)
    # Rust's float text
    specials = [0.0, -0.0, NAN, INF, -INF, MAXF, -MAXF, TINY, -TINY, 1e-300, 1e300, 0.5, 1.5, 2.5, -2.5, 0.125, 0.375,
                9.99995, 0.00005, 123456.789, 1.0, 100.0, 1e21, 1e22, 1e-7, 2.9802322387695313e-8, 0.1, 0.2, 0.3,
                1.0000000000000002, 0.9999999999999999, 4503599627370496.5, 9007199254740993.0, 3.14159, -1e-9]
    for x in specials:
        for p in (0, 1, 2, 4, 10, 17):
            row(m, "ai::fmt_fixed", "A", [f(x), i(p)])
        row(m, "ai::fmt_display", "A", [f(x)])
    for p in (0, 1, 2, 4):
        for k in (1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21, 23, 25, 99, 101, 1023, 4097):
            x = k / float(1 << (p + 1))
            row(m, "ai::fmt_fixed", "A", [f(x), i(p)])
            row(m, "ai::fmt_fixed", "A", [f(-x), i(p)])
    for _ in range(600):
        x = g.signed_log_uniform(-22.0, 22.0)
        row(m, "ai::fmt_fixed", "A", [f(x), i(4)])
        row(m, "ai::fmt_fixed", "A", [f(x), i(1)])
        row(m, "ai::fmt_display", "A", [f(x)])
    for _ in range(200):
        row(m, "ai::fmt_display", "A", [f(from_bits(g.word()))])
        row(m, "ai::fmt_fixed", "A", [f(from_bits(g.word())), i(g.int(0, 17))])
    # the data summary and the request body
    base_cols = [("series", series(40)), ("x", [1.0, 2.0, 3.0, 4.0, 5.0]), ("y", [2.1, 3.9, 6.1, 8.2, 9.8])]
    sets = [base_cols, [("b", [1.0]), ("a", []), ("C", [NAN, 1.0]), ("aa", [3.0, 3.0, 3.0])],
            [("μ", [g.signed_log_uniform(-3, 3) for _ in range(50)]), ("z`z", [1e300, -1e300]), ("", [0.5])],
            [("only", [g.uniform(-1, 1) for _ in range(200)])], [("inf", [INF, 1.0]), ("neg", [-0.0, -0.0])]]
    for cols in sets:
        row(m, "ai::build_data_summary", "A", cols_toks(cols))
    questions = ["What is the mean of x?", "", 'a "quoted" \\ back\nslash\ttab', "é 😀   \x01 \x7f", "x" * 300]
    for qn in questions:
        for model in ("llama3", "", "m\"x"):
            row(m, "ai::request_body", "A", [s(model), s(qn)] + cols_toks(base_cols))
    for cols in sets[1:]:
        row(m, "ai::request_body", "A", [s("gpt-4o"), s("summarize")] + cols_toks(cols))
    # StatQuery::query over a recorded reply
    ok = '{"id":"x","object":"chat.completion","model":"llama3","choices":[{"index":0,"message":{"role":"assistant",' \
         '"content":"The mean of x is 3.0."},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":5,' \
         '"total_tokens":15}}'
    replies = [ok, '{"choices":[{"message":{"content":"hi"}}]}', '{"choices":[{"message":{"content":null}}]}',
               '{"choices":[{"message":{}}]}', '{"choices":[]}', '{"model":null,"choices":[],"usage":null}',
               '{"choices":[{"message":{"content":"a"}},{"message":{"content":"b"}}]}',
               '{"choices":[{"message":{"content":"a"}},{"message":{"content":5}}]}', '{"model":"m"}', '{}', '[]',
               '[null,[[{"content":"seq"}]],null]', '[null,[]]', '{"choices":[{}]}', '{"choices":{}}',
               '{"choices":[{"message":{"content":"x"}}],"usage":{"prompt_tokens":-1}}',
               '{"choices":[{"message":{"content":"x"}}],"usage":{"prompt_tokens":4294967295}}',
               '{"choices":[{"message":{"content":"x"}}],"usage":{"prompt_tokens":4294967296}}',
               '{"choices":[{"message":{"content":"x"}}],"usage":{"total_tokens":1.0}}',
               '{"choices":[{"message":{"content":"x"}}],"usage":{}}', '{"choices":[{"message":{"content":"x"}}],"usage":5}',
               '{"choices":[{"message":{"content":"x"}}],"model":5}', '{"choices":[{"message":{"content":"\\u00e9\\n"}}]}',
               '{"choices":[{"message":{"content":"x"}}]} trailing', 'not json', '', '{"choices":[{"message":"x"}]}',
               '{"choices":[{"message":{"content":"x","content2":1}}],"extra":{"a":[1,2,{"b":null}]}}',
               '{"choices":[{"message":{"content":"' + "long " * 200 + '"}}]}']
    for reply in replies:
        row(m, "StatQuery::query", "A", [s("http://127.0.0.1:8088"), s("llama3"), s("What is the mean of x?"), i(200),
                                       s(reply)] + cols_toks(base_cols))
    for status in (201, 299, 300, 404, 500, 199, 0):
        row(m, "StatQuery::query", "A", [s("http://h:1"), s("m"), s("q"), i(status), s(ok)] + cols_toks(base_cols[:1]))
    for base in ("http://127.0.0.1:8088/", "http://h//", "", "https://hoosh.example/api/"):
        row(m, "StatQuery::query", "A", [s(base), s("m"), s("q"), i(200), s(ok)] + cols_toks(base_cols[1:]))
    row(m, "StatQuery::query", "A", [s("http://h"), s("m"), s("q"), i(200), s(ok)])
    for cols in sets[1:]:
        row(m, "StatQuery::query", "A", [s("http://h"), s("m"), s("q?"), i(200), s(ok)] + cols_toks(cols))
    # execute_tool
    data = [("series", series(40)), ("x", [1.0, 2.0, 3.0, 4.0, 5.0]), ("y", [2.1, 3.9, 6.1, 8.2, 9.8]),
            ("short", [1.0]), ("empty", []), ("nan", [1.0, NAN, 3.0]), ("const", [2.0, 2.0, 2.0, 2.0, 2.0]),
            ("w", [g.uniform(-5, 5) for _ in range(30)]), ("é", [0.5, 1.5, 2.5])]
    calls = [("describe", '{"column":"series"}'), ("describe", '{"column":"nope"}'), ("describe", '{}'),
             ("describe", '{"column":5}'), ("describe", '[]'), ("describe", '"x"'), ("describe", '{"column":"empty"}'),
             ("describe", '{"column":"nan"}'), ("describe", '{"column":"short"}'), ("describe", '{"column":"\\u00e9"}'),
             ("describe", '{"column":"w","extra":[1]}'), ("describe", '{"column":"const"}'),
             ("correlate", '{"column_a":"x","column_b":"y"}'), ("correlate", '{"column_a":"x","column_b":"series"}'),
             ("correlate", '{"column_a":"x","column_b":"const"}'), ("correlate", '{"column_a":"x"}'),
             ("correlate", '{"column_b":"y"}'), ("correlate", '{"column_a":"x","column_b":"nope"}'),
             ("correlate", '{"column_a":"short","column_b":"short"}'), ("correlate", '{"column_a":"w","column_b":"w"}'),
             ("regress", '{"x_column":"x","y_column":"y"}'), ("regress", '{"x_column":"x","y_column":"series"}'),
             ("regress", '{"x_column":"const","y_column":"x"}'), ("regress", '{"x_column":"x"}'),
             ("regress", '{"y_column":"x","x_column":"y"}'), ("regress", '{"x_column":"empty","y_column":"empty"}'),
             ("t_test", '{"column_a":"x","column_b":"y"}'), ("t_test", '{"column_a":"x"}'),
             ("t_test", '{"column_a":"x","mu":3}'), ("t_test", '{"column_a":"x","mu":2.5}'),
             ("t_test", '{"column_a":"x","mu":-1}'), ("t_test", '{"column_a":"x","mu":-0}'),
             ("t_test", '{"column_a":"x","mu":1e21}'), ("t_test", '{"column_a":"x","mu":"3"}'),
             ("t_test", '{"column_a":"x","mu":null}'), ("t_test", '{"column_a":"x","mu":true}'),
             ("t_test", '{"column_a":"x","mu":18446744073709551616}'), ("t_test", '{"column_a":"x","mu":0.1}'),
             ("t_test", '{"column_a":"x","column_b":5}'), ("t_test", '{"column_a":"x","column_b":"nope"}'),
             ("t_test", '{"column_a":"short"}'), ("t_test", '{"column_a":"const"}'),
             ("t_test", '{"column_a":"const","column_b":"const"}'), ("t_test", '{"column_a":"w","column_b":"series"}'),
             ("t_test", '{"column_b":"x"}'), ("t_test", '{"column_a":"nan"}'),
             ("forecast", '{"column":"series","steps":3}'), ("forecast", '{"column":"series","steps":null}'),
             ("forecast", '{"column":"series"}'), ("forecast", '{"column":"series","steps":1000000000000}'),
             ("forecast", '{"column":"series","steps":0}'), ("forecast", '{"column":"series","steps":-3}'),
             ("forecast", '{"column":"series","steps":"many"}'), ("forecast", '{"column":"series","ar_order":10000}'),
             ("forecast", '{"column":"series","steps":3.0}'), ("forecast", '{"column":"series","steps":1000}'),
             ("forecast", '{"column":"series","steps":1001}'), ("forecast", '{"column":"series","ar_order":32}'),
             ("forecast", '{"column":"series","ar_order":33}'), ("forecast", '{"column":"series","ar_order":3,"steps":12}'),
             ("forecast", '{"column":"short"}'), ("forecast", '{"column":"const"}'), ("forecast", '{"column":"x"}'),
             ("forecast", '{"column":"nope","steps":0}'), ("forecast", '{"column":"series","steps":0,"ar_order":0}'),
             ("forecast", '{"column":"w","ar_order":2,"steps":7}'), ("forecast", '{"column":"series","ar_order":-0}'),
             ("rm_rf", '{}'), ("", '{}'), ("Describe", '{"column":"x"}'), ("describe ", '{"column":"x"}')]
    for name, args in calls:
        row(m, "ai::execute_tool", "C" if name == "t_test" else "A", [s(name), s(args)] + cols_toks(data))
    return m
