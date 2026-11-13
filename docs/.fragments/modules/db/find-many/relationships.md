```tyrus
import { DbError } from "tyrus:db";

@model("teams")
export interface Team {
    @id
    id: i64;

    name: string;
}

@model("roles")
export interface Role {
    @id
    id: i64;

    name: string;
}

@model("users")
export interface User {
    @id
    id: i64;

    @foreignKey("teams", "id")
    teamId: i64?;

    @belongsTo("teamId")
    team: Team?;

    @manyToMany("user_roles", "user_id", "role_id")
    roles: Role[]?;
}

async function listUsers(): Promise<Result<User[], DbError>> {
    const result = await User.findMany({}, { include: ["team"] });
    return result;
}

interface UserRoleNames {
    id: i64;
    teamId: i64?;
    team: Team?;
    roles: { name: string }[]?;
}

async function listUserRoleNames(): Promise<Result<UserRoleNames[], DbError>> {
    const data = await User.findMany({}, {
        include: [{ roles: ["name"] }],
        relationLimit: 100
    }) onfail {
        return Err(data);
    };
    return Ok(data);
}
```
