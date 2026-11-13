```tyrus
interface User { name: string; nickname?: string; }

const user: User = { name: "Ada" };
const text: string = user.toString(); // "{ name: \"Ada\", nickname: None }"
```
