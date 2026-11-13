```tyrus
interface Example {
    @optional()
    nickName: string?;
}

const validWithValue: Example = { nickName: "Johnny" };
const validAbsent: Example = {};
```
