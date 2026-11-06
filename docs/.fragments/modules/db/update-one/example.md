```tyrus
import { DbError } from "tyrus:db";
import { User } from "./models/User";

async function updateUser(): Promise<Result<User?, DbError>> {
    const data = await User.updateOne({ id: 1 }, { email: "ada@example.com" }) onfail {
        return Err(data);
    };
    return Ok(data);
}
```
