```tyrus
import { DbError } from "tyrus:db";
import { User } from "./models/User";

async function updateExampleUsers(): Promise<Result<u64, DbError>> {
    const data = await User.updateMany(
        { email: { like: "%@example.com" } },
        { displayName: "Member" }
    ) onfail {
        return Err(data);
    };
    return Ok(data);
}
```
