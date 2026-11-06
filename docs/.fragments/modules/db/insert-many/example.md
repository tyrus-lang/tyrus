```tyrus
import { DbError } from "tyrus:db";
import { User } from "./models/User";

async function createUsers(): Promise<Result<User[], DbError>> {
    const data = await User.insertMany([
        { email: "ada@example.com" },
        { email: "grace@example.com" },
    ]) onfail {
        return Err(data);
    };
    return Ok(data);
}
```
