```tyrus
interface User {
    id: i32;
    name: string;
    email: string;
}

type UserId = PickByType<User, i32>;
const bad: UserId = { name: "Ada" }; // [!code error]
```

```text
error[E2353]: object literal specifies unknown property `name`
at /src/main.tyr:8:23
const bad: UserId = { name: "Ada" };
                      ^^^^
```
