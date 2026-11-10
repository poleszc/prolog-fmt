"""Differential test on randomly generated Prolog programs.

Usage: generated.py RUST_BIN DUCK_BIN COUNT [SEED]"""
import random, subprocess, sys, os
RUST, DUCK, N = sys.argv[1], sys.argv[2], int(sys.argv[3])
rnd = random.Random(int(sys.argv[4]) if len(sys.argv) > 4 else 1)
atoms = ['foo', 'bar', 'x', 'źdźbło', 'some_really_long_predicate_name_here', "'quoted atom'", "'it''s'", '[]', '{}', '!', 'nl', 'true']
vars_ = ['X', 'Y', 'Zs', '_', '_Acc', 'Łódź', 'Result']
nums = ['0', '42', '-1', '3.14', '1.0e10', "0'a", "0'(", "0' ", '0x1F', '1_000']
binops = ['=', '==', 'is', '<', '>=', '=..', '+', '-', '*', '/', '//', 'mod', ':', '^', '**', '->', ';', '|', '\\=', '#=', '===>']
preops = ['\\+', '-', '+', '\\', 'dynamic', '?']
def comment():
    return rnd.choice([' % c' + str(rnd.randint(0, 9)) + '\n', ' /* b */ ', '\n% own line\n', ' /* multi\n line */ '])
def ws():
    r = rnd.random()
    if r < 0.6: return ' '
    if r < 0.75: return ''
    if r < 0.9: return '\n    '
    if r < 0.95: return comment()
    return '  \n\n  '
def term(d):
    r = rnd.random()
    if d <= 0 or r < 0.3:
        return rnd.choice(atoms + vars_ + nums + ['"str. ing"', '`bq`'])
    if r < 0.5:
        n = rnd.randint(1, 5)
        return rnd.choice(atoms[:4]) + '(' + (','+ws()).join(term(d - 1) for _ in range(n)) + ')'
    if r < 0.62:
        n = rnd.randint(0, 6)
        if n == 0: return '[]'
        tail = ('|' + ws() + term(d - 1)) if rnd.random() < 0.3 else ''
        return '[' + (','+ws()).join(term(d - 1) for _ in range(n)) + tail + ']'
    if r < 0.8:
        return term(d - 1) + ws() + rnd.choice(binops) + ws() + term(d - 1)
    if r < 0.88:
        return rnd.choice(preops) + rnd.choice([' ', '']) + term(d - 1)
    if r < 0.95:
        return '(' + ws() + body(d - 1) + ws() + ')'
    return '{' + term(d - 1) + '}'
def body(d):
    goals = [term(d) for _ in range(rnd.randint(1, 4))]
    s = (','+ws()).join(goals)
    if rnd.random() < 0.3:
        s = '(' + ws() + s + ws() + '->' + ws() + term(d) + ws() + ';' + ws() + term(d) + ws() + ')'
    return s
def clause():
    r = rnd.random()
    if r < 0.1: return ':- ' + rnd.choice(['op(700, xfx, ===>)', 'dynamic foo/1', 'use_module(library(lists))', 'op(200, xfy, [&&, \'or\'])'])
    head = term(2)
    if r < 0.35: return head
    return head + ws() + rnd.choice([':-', '-->', ':-']) + ws() + body(3)
def program():
    out = []
    for _ in range(rnd.randint(1, 8)):
        c = clause()
        end = rnd.choice(['.\n', '.\n', '.\n\n', '. % trailing\n', '.\n\n\n\n', '\n', '. '])
        out.append(c + end)
        if rnd.random() < 0.2: out.append('% top comment\n')
    s = ''.join(out)
    if rnd.random() < 0.3:
        i = rnd.randrange(len(s) + 1)
        s = s[:i] + rnd.choice(["'", '"', '(', ')', ']', '/*', '.', ',']) + s[i:]
    return s
bad = 0
for k in range(N):
    p = program().encode()
    a = subprocess.run([RUST], input=p, capture_output=True)
    b = subprocess.run([DUCK], input=p, capture_output=True, timeout=30)
    if (a.returncode, a.stdout) != (b.returncode, b.stdout):
        bad += 1
        if bad <= 5:
            f = f"{os.environ.get("TMPDIR", "/tmp")}/prolog-fmt-gfail{bad}.pl"
            open(f, 'wb').write(p)
            print("MISMATCH", f, a.returncode, b.returncode)
print(f"{N} programs, {bad} mismatches")
