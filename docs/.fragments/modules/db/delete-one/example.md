```tyrus
import { DbError } from "tyrus:db";
import { User } from "./models/User";

async function removeUser(): Promise<Result<User?, DbError>> {
    const data = await User.deleteOne({ id: 1 }) onfail {
        return Err(data);
    };
    return Ok(data);
}
```
