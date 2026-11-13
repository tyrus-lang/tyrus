```tyrus
interface User { name: string; }

const first: User = { name: "Ada" };
const second: User = { name: "Ada" };
const same: boolean = first.equals(second); // true
```
