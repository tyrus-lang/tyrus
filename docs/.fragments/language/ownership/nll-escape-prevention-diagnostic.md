```text
error[E0500]: cannot return reference to local variable `x`
at /src/main.tyr:3:12
    return &x;
           ^^
help: references cannot outlive the value they point to; return an owned value instead
```
