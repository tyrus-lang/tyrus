```typescript
import { DbError, db } from "tyrus:db";
import { User } from "./models/User";

async function renameUser(id: i64): Promise<Result<boolean, DbError>> {
    const transaction = await db.beginTransaction() onfail {
        return Err(transaction);
    };
    const updated = await User.updateOne({ id }, { name: "Grace" }, { transaction }) onfail {
        return Err(updated);
    };
    const rows = await db.query<User>("SELECT id, name FROM users WHERE id = ?", [id], { transaction }) onfail {
        return Err(rows);
    };
    print(rows[0].name);
    const user = await User.findOne({ id }, { transaction }) onfail {
        return Err(user);
    };
    print(user?.name ?? "missing");
    const committed = await db.commitTransaction(transaction) onfail {
        return Err(committed);
    };
    return Ok(committed);
}
```
