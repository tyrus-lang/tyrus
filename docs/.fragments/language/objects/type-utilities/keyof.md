```tyrus
interface Account {
    id: i32;
    email: string;
}

type Key = keyof Account;
const k: Key = "id";
```

::: danger error
```tyrus
interface Account {
    id: i32;
    email: string;
}

type Key = keyof Account;
const k: Key = "missing"; // [!code error]
```

```text
error[E0308]: type mismatch for `k`: expected `"id" | "email"`, found `string`
at /src/main.tyr:7:7
const k: Key = "missing";
      ^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
