```tyrus
type Roles = Record<"admin" | "user", boolean>;
const perms: Roles = { admin: true, user: false };
```

::: danger error
```tyrus
type Roles = Record<"admin" | "user", boolean>;
const perms: Roles = { admin: true }; // [!code error]
```

```text
error[E2741]: property `user` is missing in object literal for type `object`
at /src/main.tyr:2:7
const perms: Roles = { admin: true };
      ^^^^^
```
:::

For an open key type, `Record<string, T>` resolves to `Map<string, T>`, so lookups may be absent and map methods are available:

```tyrus
let inventory: Record<string, i32> = { apples: 3, pears: 5 };
inventory["bananas"] = 12;
const apples: i32 = inventory.get("apples") ?? 0;
const hasPears: boolean = inventory.has("pears");
```
