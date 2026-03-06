# Grift Basics

Grift is a Kernel-style Lisp with first-class operatives (fexprs).
All values live in a fixed-size arena with const-generic capacity.

## Atoms

```
42          ; number
#t #f       ; booleans
hello       ; symbol
"hello"     ; string
()          ; nil / empty list
#inert      ; inert value (side-effect returns)
#ignore     ; ignore (parameter matching)
```

## Arithmetic

```
(+ 1 2)           => 3
(* 6 7)           => 42
(- 10 3)          => 7
(/ 20 4)          => 5
(mod 10 3)        => 1
```

## Comparison

```
(=? 1 1)          => #t
(<? 1 2)          => #t
(>? 2 1)          => #t
(<=? 1 1)         => #t
(>=? 2 1)         => #t
```
