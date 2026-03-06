# Advanced Features

## String operations

```
(string-length "hello")     => 5
(string-append "hi" " " "there") => "hi there"
```

## Higher-order functions

```
(map (lambda (x) (* x x)) (list 1 2 3))
  => (1 4 9)
(filter (lambda (x) (>? x 2)) (list 1 2 3 4))
  => (3 4)
(reduce + 0 (list 1 2 3))
  => 6
```

## Recursion (tail-call optimized)

```
(define! fact
  (lambda (n)
    (if (=? n 0) 1
      (* n (fact (- n 1))))))
(fact 10)           => 3628800
```

## Boolean logic

```
(and? #t #f)        => #f
(or? #t #f)         => #t
(not? #t)           => #f
```

## Type checking

```
(number? 42)        => #t
(string? "hi")      => #t
(pair? (list 1))    => #t
(null? ())          => #t
(boolean? #t)       => #t
```
