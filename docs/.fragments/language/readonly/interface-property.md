```tyrus
interface User {
    readonly id: i32;
    name: string;
}

export function main(): void {
    let user: User = { id: 1, name: "Ada" };
    print(user.id);
    user.id = 2; // [!code error]
}
```
