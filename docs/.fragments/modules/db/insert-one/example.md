```tyrus
import { DbError } from "tyrus:db";
import { User } from "./models/User";

async function createUser(): Promise<Result<User, DbError>> {
    const data = await User.insertOne({ email: "ada@example.com" }) onfail {
        return Err(data);
    };
    return Ok(data);
}
```
