```tyrus
import { DbError, db } from "tyrus:db";

interface DbExample {
    id: i64;
    label: string;
}

async function updateExample(id: i64): Promise<Result<void, DbError>> {
    const data = await db.query("UPDATE db_examples SET label = ? WHERE id = ?", ["Ada", id]) onfail {
        return Err(data);
    };
    return Ok(data);
}

async function selectExample(id: i64): Promise<Result<DbExample[], DbError>> {
    const data = await db.query<DbExample>("SELECT * FROM db_examples WHERE id = ?", [id]) onfail {
        return Err(data);
    };
    return Ok(data);
}
```
