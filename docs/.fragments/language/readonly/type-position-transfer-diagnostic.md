```text
error[E0308]: type mismatch for `ps`: expected `i32[]`, found `readonly i32[]`
at /src/main.tyr:4:9
    let ps: i32[] = PRIMES;
        ^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
