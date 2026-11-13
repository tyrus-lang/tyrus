```tyrus
interface UserService {
    id: i32;
    name: string;
    save: () => boolean;
    notify: (string) => void;
}

type UserData = DataOnly<UserService>;
const data: UserData = { id: 1, name: "Ada" };
```
