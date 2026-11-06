```tyrus
import { DbError, db } from "tyrus:db";

interface ExampleWithRole {
    label: string;
    title: string?;
}

async function listExamplesWithRoles(): Promise<Result<ExampleWithRole[], DbError>> {
    const rows = await db.query<ExampleWithRole>("""
        SELECT e.label AS label, r.title AS title 
        FROM db_examples AS e 
        LEFT JOIN db_example_roles AS r ON e.id = r.id 
        ORDER BY e.id
    """, []) onfail {
        return Err(data);
    };
    
    return Ok(rows);
}
```
