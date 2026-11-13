```tyrus
import { DbError, db } from "tyrus:db";

interface GroupRow {
    groupId: i64;
    total: i64;
}

async function listGroups(): Promise<Result<GroupRow[], DbError>> {
    const minimumTotal: i64 = 1;
    const rows = await db.query<GroupRow>("""
        SELECT id AS groupId, COUNT(*) AS total
        FROM db_examples
        GROUP BY groupId
        HAVING total > ?
        ORDER BY total DESC
    """, [minimumTotal]) onfail {
        return Err(data);
    };
    return Ok(rows);
}
```
