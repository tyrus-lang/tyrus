```tyrus
interface User {
    id: i32;
    name: string;
    email?: string;
    password: string;
}

type Update = Optional<User, "name" | "email">;
const update: Update = { id: 1, password: "rotated" };

const user: User = { id: 1, name: "Ada", password: "secret" };
const publicUser = user.pick(["id", "name"]);
const storedUser = user.omit(["password"]);

print(publicUser.name);
print(storedUser.id);
```
