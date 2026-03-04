# Vau Calculus Explained

*2025-03-10*

Unlike traditional Lisps, Grift uses vau calculus where
operatives receive their arguments unevaluated along with
the caller's environment. This makes operatives strictly
more powerful than macros — they can choose whether and
when to evaluate each argument.

`($vau (x) env-param body)` creates an operative that
captures the formal parameter tree, environment parameter,
and body expression as a closure.
