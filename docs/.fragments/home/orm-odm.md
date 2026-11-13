```tyrus
import { User } from "./models/User";

async function main(): Promise<void> {
    const users = await User.findMany({ active: true });
}
```
