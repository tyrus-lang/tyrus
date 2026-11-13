```tyrus
import { DbError, db } from "tyrus:db";

interface CountRow {
    total: i64;
}

interface StatsRow {
    sum: i64?;
    average: f64?;
    minimum: i64?;
    maximum: i64?;
}

interface GroupRow {
    groupId: i64;
    total: i64;
}

async function countExamples(): Promise<Result<CountRow[], DbError>> {
    const rows = await db.query<CountRow>("SELECT COUNT(*) AS total FROM db_examples", []) onfail {
        return Err(data);
    };
    return Ok(rows);
}

async function readStats(): Promise<Result<StatsRow[], DbError>> {
    const rows = await db.query<StatsRow>("""
        SELECT SUM(id) AS sum,
               AVG(id) AS average,
               MIN(id) AS minimum,
               MAX(id) AS maximum
        FROM db_examples
    """, []) onfail {
        return Err(data);
    };
    return Ok(rows);
}

async function listGroups(): Promise<Result<GroupRow[], DbError>> {
    const rows = await db.query<GroupRow>("SELECT id AS groupId, COUNT(*) AS total FROM db_examples GROUP BY groupId ORDER BY total DESC", []) onfail {
        return Err(data);
    };
    return Ok(rows);
}
```
