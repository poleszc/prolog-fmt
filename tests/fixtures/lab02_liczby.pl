% nat(X) - X jest liczbą naturalną
nat(0).
nat(s(X)) :- nat(X).

% plus(X, Y, Z) wtw. X + Y = Z
plus(0, X, X) :- nat(X).
plus(s(X), Y, s(Z)) :- plus(X, Y, Z).

% minus(X, Y, Z) wtw. X - Y = Z
minus(X, Y, Z) :- plus(Y, Z, X).

% minus(X, Y, Z) wtw. X - Y = Z
minus2(X, 0, X) :- nat(X).
minus2(s(X), s(Y), Z) :- minus(X, Y, Z).

% fib(K, X) wtw. x jest k-tą liczba fibonacciego
fib(0, 0).
fib(s(0), s(0)).
fib(s(s(K)), X) :- fib(s(K), A), fib(K, B), plus(A, B, X).

% le(X, Y) wtw. X <= Y
le(0, _).
le(s(X), s(Y)) :- le(X, Y).

% lt(X, Y) wtw. X < Y
% lt(0, _).
% lt(s(X), s(Y)) :- le(X, Y).

% razy(x, y, z) wtw. x * y = z
razy(0, _, 0).
razy(s(X), Y, Z) :- razy(X, Y, A), plus(A, Y, Z).

% exp(n, x, z) wtw. x^n == z
exp(0, _, s(0)).
exp(s(X), Y, Z) :- exp(X, Y, A), razy(A, Y, Z).

% silnia(n, s) wtw. s == n!
silnia(0, s(0)).
silnia(s(X), Y) :- silnia(X, Z), razy(Z, s(X), Y).

% odd(n) wtw. jest niepatrzysta
odd(s(0)).
odd(s(s(X))) :- odd(X).

% even(n) wtw. jest patrzysta
even(s(X)) :- odd(X).

% even2(n) wtw. jest patrzysta
even2(X) :- \+ odd(X).

% div(X, Y, N) wtw. X / Y = N
div(X, Y, N) :- razy(N, Y, X).

% mod(x, y, z) wtw. x % y = z
mod(X, Y, X) :- lt(X, Y).
mod(X, Y, Z) :- minus(X, Z, Res), razy(Res, Y, Z).
