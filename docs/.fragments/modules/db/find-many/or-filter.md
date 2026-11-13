```tyrus
import { DbError } from "tyrus:db";
import { User } from "./models/User";

async function findAdministrators(): Promise<Result<User[], DbError>> {
    const data = await User.findMany({
        active: true,
        role: { like: "admin%" },
        or: [
            { age: { gte: 18 } },
            { id: { in: [1, 2, 3] } }
        ]
    }) onfail {
        return Err(data);
    };
    return Ok(data);
}
```
