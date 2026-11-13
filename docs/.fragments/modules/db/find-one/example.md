```tyrus
import { DbError } from "tyrus:db";
import { User } from "./models/User";

async function findUser(): Promise<Result<Optional<User, void>, DbError>> {
    const data = await User.findOne({ id: 1 }) onfail {
        return Err(data);
    };
    return Ok(data);
}
```
