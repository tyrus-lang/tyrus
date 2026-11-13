```tyrus
interface User {
    id: i32;
    name: string;
    email: string;
}

type PublicUser = Omit<User, "email">;
const pub: PublicUser = { id: 1, name: "Ada" };
```

```tyrus
interface User {
    id: i32;
    name: string;
    email: string;
}

type PublicUser = Omit<User, "email">;
const email = pub.email; // [!code error]
```

```text
error[E0411]: field `email` does not exist on `PublicUser`
  --> src/main.tyr:6:21
   |
 6 | const email = pub.email;
   |                     ^^^^^ field `email` was omitted from the original type
   |
help: use the original `User` type if `email` is needed
```
