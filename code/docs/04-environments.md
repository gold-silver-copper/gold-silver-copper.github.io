# Environments & Evaluation

Environments are first-class in Grift:

```
(get-current-environment)  => <environment>
(make-environment)         => <empty-env>
(eval expr env)            => evaluate expr in env
```

## Operatives receive the dynamic environment

```
($vau (x) e (eval x e))   ; like lambda
(wrap ($vau (x) #ignore x)) ; applicative from operative
```

## The evaluator

1. Symbols are looked up in the current environment
2. Pairs: evaluate the operator, then combine
3. Operatives receive operands unevaluated
4. Applicatives evaluate operands first, then call

## Tail-call optimization

Grift optimizes tail positions so recursive functions
run in constant stack space. This applies to if, cond,
begin, let, and operative/applicative bodies.
