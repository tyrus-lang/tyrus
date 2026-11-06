```tyrus
interface User {
    name: string;
    email: string;
}

const users: Map<i32, User> = Map<i32, User>();
users.set(1, { name: "Ada", email: "ada@example.com" });
users.set(2, { name: "Grace", email: "grace@example.com" });

const first: User? = users.get(1);
print(first?.name ?? "unknown");       // "Ada"

const exists: boolean = users.has(2);        // true
const size: usize = users.size;              // 2

for (const [id, user] of users.entries()) {
    print(`${id}: ${user.name}`);
    // 1: Ada
    // 2: Grace
}

const visits = users.getOrInsert(1, { name: "Ada", email: "ada@example.com" });
// The fallback expression is evaluated only when key `1` is absent.
```
