```tyrus
type User =
    | { kind: "admin"; name: string; permissions: string }
    | { kind: "guest"; name: string; expiresAt: i64 };

const nameOf = (user: User): string => {
    match (user) {
        ({ kind: "admin", name }) => { return name; }
        ({ kind: "guest", name }) => { return name; }
    }
    return "";
};

const users: User[] = [
    { kind: "admin", name: "Ada", permissions: "root" },
    { kind: "guest", name: "Bob", expiresAt: 999 },
];
```
