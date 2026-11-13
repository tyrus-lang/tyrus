```tyrus
interface User { name: string; }

const user: User = { name: "Ada" };
const copied: User = user.copy();
print(copied); // { name: "Ada" }
```
