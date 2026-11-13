```tyrus
import { DbError, Page } from "tyrus:db";
import { User } from "./models/User";

async function listNextUsers(after: string): Promise<Result<Page<User>, DbError>> {
    const data = await User.findMany({}, { paginate: true, pageSize: 25, cursor: true, after }) onfail {
        return Err(data);
    };
    return Ok(data);
}
```
