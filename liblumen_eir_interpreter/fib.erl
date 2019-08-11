-module(fib).

-export([run/0]).

run() ->
    N = fib(8).

fib(0) -> 0;
fib(1) -> 1;
fib(X) -> fib(X-1) + fib(X-2).
