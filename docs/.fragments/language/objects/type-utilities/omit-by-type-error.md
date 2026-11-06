```tyrus
interface User {
    id: i32;
    name: string;
    email: string;
}

type UserData = OmitByType<User, string>;
const bad: UserData = { name: "Ada" }; // [!code error]
```

```text
error[E2353]: object literal specifies unknown property `name`
at /src/main.tyr:8:25
const bad: UserData = { name: "Ada" };
                        ^^^^
```
