:- module(foo, [bar/2,   baz/1]).
:- use_module(library(lists)).
:- dynamic counter/1.
:- op(700, xfx, ===>).

max(X,Y,Z):-(X>=Y->Z=X;Z=Y).
classify(X, C) :- ( X < 0 -> C = neg ; X =:= 0 -> C = zero ; C = pos ).
p(X) :- a(X),   % first goal
    % commented out: b(X),
    c(X).   % last
q :- \+ member(x, [a,b,c]), X is - 1, Y is -1, Z = -(1), W = - (a,b), K-V = a-b, M = lists:append.
r(X) :- X ===> y.
greeting --> [hello], name, { write(done) }.
name --> [world] | [prolog].
long_fact(aaaaaaaaaaaa, bbbbbbbbbbbbbbbbbbb, cccccccccccccccccccccc, dddddddddddddddddddddd, eeeeeeeeeeeeeeeeeeeee, fffffffffff, gggggg).
nums([1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38,39,40,41,42,43]).
s :- findall(X-Y, (member(X, [1,2,3]), member(Y, [a,b]), X > 1), L), forall(member(P, L), (print(P), nl)).
str("hello \"world\"", 'it''s', 0' , 0'a, `back`, 0x1F, 1.5e10).
nested(X) :- ( X = a -> ( foo, bar ; baz ) ; qux ).
disj_body(X) :- a(X) ; b(X).
/* block
   comment */
t(X) :- X = "a. b", Y = 'end.', Z = f(
   a,   b).
