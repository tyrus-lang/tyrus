```tyrus
import { DbError } from "tyrus:db";
import { User } from "./models/User";

async function countActiveUsers(): Promise<Result<u64, DbError>> {
    const data = await User.count({ active: true }) onfail {
        return Err(data);
    };
    return Ok(data);
}

async function countAllUsers(): Promise<Result<u64, DbError>> {
    const data = await User.count({}) onfail {
        return Err(data);
    };
    return Ok(data);
}
```
