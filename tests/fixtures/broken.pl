good(X) :- foo(X),bar(X).
missing_paren(X) :- foo(X, bar(X), baz.
next_ok(Y):-   y(Y).
missing_dot(X) :- a(X)
also_merged(Y) :- b(Y).
bad_op(X) :- X = = Y,
        foo(  [1,2
   |T]).
unterminated('abc) :- x.
after_unterminated  :-  fine.
partial(X) :-
    a(X),
    b(X
