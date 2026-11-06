```text
error[E0384]: cannot assign through immutable `config`
at /src/main.tyr:6:5
    config.port = 9090;
    ^^^^^^
help: declare `config` with `let` to allow assignment
error[E2541]: cannot assign to `port` through a readonly value
at /src/main.tyr:6:5
    config.port = 9090;
    ^^^^^^^^^^^^^^^^^^^
error[E0500]: cannot assign through immutable `config` (declared with `const`)
at /src/main.tyr:6:5
    config.port = 9090;
    ^^^^^^
help: declare `config` with `let` to allow mutation
```
