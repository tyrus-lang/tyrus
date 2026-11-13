```tyrus
interface ApiUser {
    user_id: i32;
    first_name: string;
    is_active: boolean;
}

type User = CamelCaseKeys<ApiUser>;
const user: User = { userId: 1, firstName: "Ada", isActive: true };
```
