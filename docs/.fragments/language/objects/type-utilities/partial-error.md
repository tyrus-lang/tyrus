```tyrus
interface Account {
    id: i32;
    email: string;
    active: boolean;
}

type AccountPatch = Partial<Account>;
const patch: AccountPatch = { email: 42 }; // [!code error]
```

```text
error[E0308]: type mismatch for `email`: expected `string?`, found `42`
at /src/main.tyr:8:31
const patch: AccountPatch = { email: 42 };
                              ^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
