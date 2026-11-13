```tyrus
import { DbError } from "tyrus:db";
import { User } from "./models/User";

async function removeExampleUsers(): Promise<Result<u64, DbError>> {
    const data = await User.deleteMany({ email: { like: "%@example.com" } }) onfail {
        return Err(data);
    };
    return Ok(data);
}
```
