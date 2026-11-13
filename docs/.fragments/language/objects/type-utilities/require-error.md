```tyrus
interface User {
    id: i32;
    name: string;
    email: string;
}

type Strict = Partial<User> & Require<User, "id">;
const bad: Strict = { name: "Ada" }; // [!code error]
```

```text
error[E0401]: missing required field `id`
  --> src/main.tyr:7:24
   |
 7 | const bad: Strict = { name: "Ada" };
   |                        ^^^^^^^^^^^^^^ field `id` is required by `Require<User, "id">`
   |
help: add `id` to the object literal
```
