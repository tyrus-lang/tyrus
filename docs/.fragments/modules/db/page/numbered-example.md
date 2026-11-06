```tyrus
import { DbError, Page } from "tyrus:db";
import { User } from "./models/User";

async function listUsersPage(page: u64): Promise<Result<Page<User>, DbError>> {
    const data = await User.findMany({}, { paginate: true, pageSize: 25, page }) onfail {
        return Err(data);
    };
    return Ok(data);
}

// Example result for page 1:
// {
//     items: [{ id: 1, name: "Ada" }, ...],
//     total: 64,
//     pageSize: 25,
//     page: 1,
//     totalPages: 3
// }
```
