```tyrus
import { DbError, Page } from "tyrus:db";
import { User } from "./models/User";

async function listUsersPage(): Promise<Result<Page<User>, DbError>> {
    const data = await User.findMany({}, { paginate: true, pageSize: 25, page: 1 }) onfail {
        return Err(data);
    };
    return Ok(data);
}
```

Cursor pagination uses the same `Page<T>` result type. `nextCursor` contains the serialized `@id` value of the last returned row, not an item count or page number; omit `after` for the first page and pass `nextCursor` back unchanged to continue. `total` still counts every record matching the filter, before the cursor position.

<!-- @include: cursor-pagination.md -->
