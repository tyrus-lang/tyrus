```tyrus
interface UserInput {
    @minLength(3)
    name: string;
}

const input: UserInput = { name: "Al" }; // [!code error]
```
