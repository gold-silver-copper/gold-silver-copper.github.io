# Error Handling & Debugging

Grift reports errors as readable messages:

```
(/ 1 0)               => Error: DivisionByZero
(car 42)              => Error: TypeMismatch
undefined-sym          => Error: UnboundSymbol
```

## Common errors

- **TypeMismatch** — wrong argument type
- **ArityMismatch** — wrong number of arguments
- **UnboundSymbol** — symbol not defined in scope
- **DivisionByZero** — division by zero
- **ArenaFull** — arena capacity exceeded

## Debugging tips

1. Check types with predicates: number?, pair?, string?
2. Inspect environments with get-current-environment
3. Use begin to sequence debug prints
4. Break complex expressions into smaller define! steps
