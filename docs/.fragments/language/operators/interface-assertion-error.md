```tyrus
interface Loose {
    age: i32;
}

interface User {
    @min(18)
    age: i32;
}

const loose: Loose = { age: 1 };
const user: User = loose as User;   // [!code error]
```
