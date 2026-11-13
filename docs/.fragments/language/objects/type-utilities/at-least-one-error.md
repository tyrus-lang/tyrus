```tyrus
interface User {
    id: i32;
    name: string;
    email: string;
}

type Lookup = AtLeastOne<User, "name" | "email">;
const bad: Lookup = { id: 1 }; // [!code error]
```

```text
error[E0308]: type mismatch for `bad`: expected `{ id: i32; name: string; email?: string } | { id: i32; name?: string; email: string }`, found `object`
at /src/main.tyr:8:7
const bad: Lookup = { id: 1 };
      ^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
