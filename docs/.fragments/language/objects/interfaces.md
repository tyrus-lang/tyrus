```tyrus
interface User {
    readonly id: i32;
    name: string;
    nickname?: string;
    active: boolean;
}

const user: User = {
    id: 1,
    name: "Ada",
    active: true,
};

const renamed: User = {
    ...user,
    name: "Grace",
};

const active: boolean = true;
const shorthand: User = { id: 2, name: "Grace", active };
```
