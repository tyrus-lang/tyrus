```tyrus
// The default PostgreSQL identity ID is added to the Model automatically.
import { Instant } from "tyrus:temporal";

@model("users")
export interface User {
    @email()
    @unique
    email: string;

    @default("Anonymous")
    displayName: string;

    @column("team_id")
    @foreignKey("teams", "id")
    teamId: i64?;

    deletedAt: Instant?;
}
```
