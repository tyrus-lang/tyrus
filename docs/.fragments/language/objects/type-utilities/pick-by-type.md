```tyrus
interface User {
    id: i32;
    name: string;
    email: string;
    active: boolean;
}

type UserStrings = PickByType<User, string>;
const preview: UserStrings = { name: "Ada", email: "ada@example.com" };
```
