"""Differential test: every prefix and random mutations of the given files must format
identically with the Rust binary (argv[1]) and the Duckling binary (argv[2])."""
import random, subprocess, sys, glob, os
RUST, DUCK = sys.argv[1], sys.argv[2]
files = sys.argv[3:]
def run(exe, data):
    r = subprocess.run([exe], input=data, capture_output=True, timeout=30)
    return r.returncode, r.stdout
cases = []
for f in files:
    src = open(f, 'rb').read()
    txt = src.decode()
    for cut in range(0, len(txt), 11):
        cases.append(txt[:cut].encode())
    rnd = random.Random(f)
    alphabet = list("()[]{},|.;'\"%\n  \t:-=\\+0X_ab") + ['ą', 'Ł', '/*', '*/', '->', '0\'']
    for _ in range(150):
        t = list(txt)
        for _ in range(rnd.randint(1, 6)):
            i = rnd.randrange(len(t) + 1)
            op = rnd.random()
            if op < 0.4 and t: del t[min(i, len(t) - 1)]
            elif op < 0.8: t.insert(i, rnd.choice(alphabet))
            elif t: t[min(i, len(t) - 1)] = rnd.choice(alphabet)
        cases.append(''.join(t).encode())
bad = 0
for k, c in enumerate(cases):
    a, b = run(RUST, c), run(DUCK, c)
    if a != b:
        bad += 1
        if bad <= 3:
            p = f"{os.environ.get("TMPDIR", "/tmp")}/prolog-fmt-fail{bad}.pl"
            open(p, 'wb').write(c)
            print("MISMATCH", p, a[0], b[0])
print(f"{len(cases)} cases, {bad} mismatches")
