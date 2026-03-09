# Special Forms & Definitions

## Define variables

```
(define! x 42)
x                 => 42
```

## Lambda (applicative)

```
(define! double (lambda (x) (* x 2)))
(double 21)       => 42
```

## Conditionals

```
(if #t 1 2)       => 1
(if #f 1 2)       => 2
(cond (#f 1) (#t 2))  => 2
```

## Lists

```
(list 1 2 3)      => (1 2 3)
(cons 1 (list 2)) => (1 2)
(car (list 1 2))  => 1
(cdr (list 1 2))  => (2)
```

## Operatives (vau / fexprs)

```
($vau (x) e x)    ; raw operative
(wrap ($vau (x) #ignore x)) ; applicative
```

## Let bindings

```
(let ((x 1) (y 2)) (+ x y)) => 3
```

## Sequencing

```
(begin (define! a 1) (+ a 2)) => 3
```
