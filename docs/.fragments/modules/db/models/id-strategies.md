For SQLite, MongoDB, and the default MySQL and PostgreSQL strategies, declare the Model without an ID field:

```tyrus
@model("users")
export interface User {
    name: string;
}
```

| Backend | Default strategy | Source Model | Generated ID in Model results |
| --- | --- | --- | --- |
| SQLite | `INTEGER PRIMARY KEY` / rowid | No ID field | Synthetic readonly `id: i64` |
| MongoDB | Server-generated `_id` | No `_id` field | Synthetic readonly [`_id: ObjectId`](#objectid) |
| MySQL | `auto_increment` | No ID field | Synthetic readonly `id: i64` |
| PostgreSQL | `identity` | No ID field | Synthetic readonly `id: i64` |

Only declare an ID field when selecting a non-default MySQL or PostgreSQL strategy. Each declaration below uses the same simple `User` Model; import [`Uuid`](#uuid) or [`Uuid7`](#uuid7) from `tyrus:db` as shown.

MySQL `uuid` uses `Uuid`:

```tyrus
import { Uuid } from "tyrus:db";

@model("users")
export interface User {
    @id({ strategy: "uuid" })
    id: Uuid;
    name: string;
}
```

MySQL `uuid_short` uses `u64`:

```tyrus
@model("users")
export interface User {
    @id({ strategy: "uuid_short" })
    id: u64;
    name: string;
}
```

PostgreSQL `uuid` uses `Uuid`:

```tyrus
import { Uuid } from "tyrus:db";

@model("users")
export interface User {
    @id({ strategy: "uuid" })
    id: Uuid;
    name: string;
}
```

PostgreSQL `uuid7` uses `Uuid7`:

```tyrus
import { Uuid7 } from "tyrus:db";

@model("users")
export interface User {
    @id({ strategy: "uuid7" })
    id: Uuid7;
    name: string;
}
```
