```text
error[E0605]: cannot convert `Loose` to `User` with `as`: `as` converts, it does not reinterpret: a `Loose` value is not a `User`
at /src/main.tyr:11:20
const user: User = loose as User;
                   ^^^^^
help: build the target value from the source instead of asserting it, or annotate the value with the target type where it is created
```
