```tyrus
import { DbError, Page } from "tyrus:db";
import { User } from "./models/User";

async function listFirstUsersCursorPage(): Promise<Result<Page<User>, DbError>> {
    const data = await User.findMany({}, { paginate: true, pageSize: 25, cursor: true }) onfail {
        return Err(data);
    };
    return Ok(data);
}

// Example result when another page is available:
// {
//     items: [{ id: 1, name: "Ada" }, ...],
//     total: 64,
//     pageSize: 25,
//     hasMore: true,
//     nextCursor: "25"
// }

async function listNextUsersCursorPage(after: string): Promise<Result<Page<User>, DbError>> {
    const data = await User.findMany({}, { paginate: true, pageSize: 25, cursor: true, after }) onfail {
        return Err(data);
    };
    return Ok(data);
}

// Example result for the next page:
// {
//     items: [{ id: 26, name: "Grace" }, ...],
//     total: 64,
//     pageSize: 25,
//     hasMore: true,
//     nextCursor: "50"
// }
// On the final page, hasMore is false and nextCursor is omitted.
```
