% -------- Zadanie 1 --------
% lista(X), wtw gdy X jest listą.
lista([]).
lista([_ | T]) :- lista(T).

% pierwszy(E, X), wtw gdy E jest pierwszym elementem listy
pierwszy(H, [H | _]).

% ostatni(E, X), wtw gdy E jest ostatnim elementem listy
ostatni(E, [E]).
ostatni(E, [_ | T]) :- ostatni(E, T).

% element(E, X), wtw gdy E jest elementem listy
element(E, [E | _]).
element(E, [_ | T]) :- element(E, T).

% intersection(L1, L2), wtw gdy zbiory L1 i L2 nie są rozłączne.
% istnieje element w L1, który jest też w L2.
intersection(L1, L2) :- element(H, L1), element(H, L2). 

% rowne(L1, L2), wtw gdy L1 == L2
rowne([], []).
rowne([H | T1], [H | T2]) :- rowne(T1, T2).

% scal(L1, L2, R), wtw gdy L1 + L2 == L3.
% scal([H1 | T1], L2, R) :- rowne([H1 | scal(T1, L2)], R).
% scal([H1 | T1], [H2 | T2], R) :- [H1 | (T1 | H2 | T2)] == R.
scal([], L, L).
scal([X | T], P, [X | R]) :- scal(T, P, R).

% -------- Zadanie 2 --------
% sufiks(S, L), wtw gdy S jest sufiksem L.
sufiks(S, L) :- scal(_P, S, L).

% prefiks(P, L), wtw gdy P jest prefiksem L.
prefiks(P, L) :- scal(P, _S, L).

% first(E, L), wtw gdy E jest pierwszym elementem L.
first(E, L) :- prefiks([E], L).

% last(E, L), wtw gdy E jest ostatim elementem L.
last(E, L) :- sufiks([E], L).

% removeLast(L1, L2), wtw gdy L2 jest listą L1 bez ostatniego elementu.
removeLast(L1, L2) :- scal(L2, [_], L1).

% -------- Zadanie 3 --------
% podziel(Lista, NieParz, Parz), wtw gdy:
%   - NieParz jest listą zawierającą wszystkie elementy 
%     listy znajdującą się na indeksach niepatzystych.
%   - Parz jest listą zawierającą wszystkie elementy 
%     listy znajdującą się na indeksach parzystych.
podziel([], [], []).
podziel([A], [A], []).
podziel([A, B | T], [A | T1], [B | T2]) :- podziel(T, T1, T2).

podziel2([], [], []).
podziel2([A | T], [A | T1], L2) :- podziel(T, L2, [A | T1]).

% czytelne wypisanie wszystkich elementów listy L
wypiszInternal([]) :- write("Pusta").
wypiszInternal([A]) :- write(A), write("]").
wypiszInternal([A | T]) :- write(A), write(", "), wypiszInternal(T).
wypisz(L) :- write("["), wypiszInternal(L).

% dlugosc(L, N), wtw gdy N to długość L.
dlugosc([], 0).
dlugosc([_], 1).
dlugosc([_ | T], N) :- dlugosc(T, N - 1).

% rownaDlugosc(L1, L2), wtw gdy L1 i L2 mają taką samą długość.
rownaDlugosc([], []).
rownaDlugosc([_ | T1], [_ | T2]) :- rownaDlugosc(T1, T2).

% srodek(E, L) wtw, gdy E jest środkowym elementem listy L  o nieparzystej długości,
% czyli (k+1)-ym elementem, gdzie 2k+1 = długość listy L
srodek(E, L) :- scal(P, [E|S], L), rownaDlugosc(P, S).

% Ten jest liniowy
srodek2(E, L) :- srodek2(E, L, L).
srodek2(E, [E | _], [_]).
srodek2(E, [_ | L1], [_, _ | L2]) :- srodek2(E, L1, L2).

podlista(B, ABC) :- scal(_A, BC, ABC), scal(B, _C, BC).


% 1. Dana definicja predykatu nawiasy/2:
%        nawiasy(NawiasOtwierający, NawiasZamykający)
%    Zdefiniuj predykat wyrNawiasowe(Wyrażenie) odnoszący sukces wtw. gdy
%    podane Wyrażenie (lista) jest poprawnym wyrażeniem nawiasowym.

%    Na przykład:
%         nawiasy(0'(, 0')).      % definicja nawiasów
%    nawiasy(0'[, 0']).

%    ?- wyrNawiasowe("([()])").    sukces
%    ?- wyrNawiasowe("([[)[])").   porażka
%    ?- wyrNawiasowe("{}").        porażka

% Potrzeba ustawić to w REPLu:
% set_prolog_flag(double_quotes, codes).
nawiasy(0'(, 0')).
nawiasy(0'[, 0']).
nawiasy(0'{, 0'}).
nawiasy(0'<, 0'>).

wyrNawiasowe(L) :- wyrNawiasowe(L, []).
wyrNawiasowe([], []).
wyrNawiasowe([NO | R], S) :- nawiasy(NO, NZ), wyrNawiasowe(R, [NZ | S]).
wyrNawiasowe([NZ | R], [NZ | RS]) :- wyrNawiasowe(R, RS).

% 2. Zdefiniuj predykat qsort(L, S) implementujący algorytm QuickSort.
%     a) wersja bez akumulatora (definicja algorytmu),
%     b) wersja z akumulatorem.

% [L < Pivot | R < Pivot] &&
% partition(List, Pivot, L, R).
partition([], _, [], []).
partition([H | T], X, [H | LM], LW) :- 
    H =< X, 
    partition(T, X, LM, LW).
partition([H | T], X, LM, [H | LW]) :- 
    H > X, 
    partition(T, X, LM, LW).

% qsort(List, SortedList).
qsort([], []).
qsort([L | Ls], S) :- 
    partition(Ls, L, X, Y), 
    qsort(X, XS), 
    qsort(Y, YS), 
    append(XS, [L | YS], S).


% qsort2(List, Result)
qsort2(L, Res)  :- qsort2(L, [], Res).

% qsort2(List, Acc, Result)
qsort2([], S, S). % Już wynik w akumulatorze
qsort2([X | L], Acc, Res) :- 
    partition(L, X, LM, LW),
    qsort2(LW, Acc, RightRes),
    qsort2(LM, [X | RightRes], Res).

% 3. Zdefiniuj predykat flatten(LL, L) odnoszący sukces wtw, gdy lista LL jest zagnieżdżoną listą list,
%      której elementami są liczby całkowite, a lista L jest płaską listą wszystkich liczb całkowitych z listy LL (w tej samej kolejności).
%    Np. ?- flatten([[[[1,[2,[3],4],5],6],[]],7], L).
%    L = [1,2,3,4,5,6,7];
%    no
flatten(List, Res) :- flatten(List, [], Res).
flatten([], X, X).
flatten(X, L, [X | L]) :- integer(X).
flatten([L | Ls], Acc, Res) :- 
    flatten(L, FlattenedL, Res),
    flatten(Ls, Acc, FlattenedL).
















% 4. Zdefiniuj predykat qsort/2 tak, aby głębokość stosu rekurencyjnych
%    wywołań qsort była logarytmiczna (oczywiście bez append/3).