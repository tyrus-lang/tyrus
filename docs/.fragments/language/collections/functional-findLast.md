```tyrus
const users: string[] = ["Ada", "Grace", "Linus", "Gordon", "Margaret"];
const last: string? = users.findLast(name => name.startsWith("G"));
// "Gordon"
```

::: danger error
```tyrus
const users: string[] = ["Ada", "Grace", "Linus", "Gordon", "Margaret"];
const last: string = users.findLast(name => name.startsWith("G")); // [!code error]
```

```text
error[E0308]: type mismatch for `last`: expected `string`, found `string?`
at /src/main.tyr:2:7
const last: string = users.findLast(name => name.startsWith("G"));
      ^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
