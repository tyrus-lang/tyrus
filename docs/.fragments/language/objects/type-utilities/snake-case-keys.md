```tyrus
interface User {
    userId: i32;
    firstName: string;
    isActive: boolean;
}

type UserDb = SnakeCaseKeys<User>;
const row: UserDb = { user_id: 1, first_name: "Ada", is_active: true };
```
