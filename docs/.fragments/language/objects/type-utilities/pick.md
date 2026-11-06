```tyrus
interface User {
    id: i32;
    name: string;
    email: string;
    role: string;
}

type UserSummary = Pick<User, "id" | "name">;
const summary: UserSummary = { id: 1, name: "Ada" };
```

::: danger error
```tyrus
interface User {
    id: i32;
    name: string;
    email: string;
}

type Bad = Pick<User, "id" | "missing">; // [!code error]
```

```text
error[utility-unknown-key]: `missing` is not a property of the type given to `Pick`
at /src/main.tyr:1:1
interface User {
^
```
:::
