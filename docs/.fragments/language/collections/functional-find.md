```tyrus
const users: string[] = ["Ada", "Grace", "Linus", "Gordon", "Margaret"];
const match: string? = users.find(name => name.startsWith("G"));
// "Grace"
```

::: danger error
```tyrus
const users: string[] = ["Ada", "Grace", "Linus", "Gordon", "Margaret"];
const match: string = users.find(name => name.startsWith("G")); // [!code error]
```

```text
error[E0003]: `match` is a reserved keyword in tyrus and cannot be used as an identifier
at /src/main.tyr:2:7
const match: string = users.find(name => name.startsWith("G"));
      ^^^^^
help: rename `match` to an identifier that is not a reserved keyword
```
:::
