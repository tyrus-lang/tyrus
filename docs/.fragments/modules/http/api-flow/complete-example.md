### `tyrus.json`

```json
{
  "project": {
    "name": "api-sqlite",
    "version": "0.1.0",
    "authors": [],
    "description": "",
    "target": "api"
  },
  "scripts": {
    "fmt": "tyrus fmt",
    "lint": "tyrus lint"
  },
  "security": {
    "envFile": ".env",
    "envSealed": false
  },
  "imports": {
    "#": "./src"
  },
  "database": {
    "backend": "sqlite",
    "path": "./data/app.db",
    "sqlite": {
      "journalMode": "wal",
      "synchronous": "full",
      "busyTimeoutMs": 5000
    }
  }
}
```

### `src/main.tyr`

```tyrus
import { HttpRouter, HttpServer, createRouter } from "tyrus:http";

const main = async () => {
    const app: HttpRouter = createRouter();

    app.docs(
        {
            title: "SQLite Users API",
            version: "0.1.0",
            description: "User endpoints backed by SQLite.",
        },
    );

    const server: HttpServer = app.listen(3000) onfail {
        printError(`${server.code}: ${server.message}`);
    };

    print(`Server listening on port ${server.port()}`);
};
```

### `src/models/users.tyr`

```tyrus
@model("users")
export interface User {
    @minLength(3)
    name: string;
}
```

### `src/schemas/user.tyr`

```tyrus
import { User } from "#/models/users";

export interface UserParams {
    @positive()
    id: i64;
}

export interface CreateUserBody {
    @minLength(1)
    name: string;
}

export interface PatchUserBody {
    @minLength(3)
    name: string;
}

export interface ReplaceUserBody {
    @minLength(3)
    name: string;
}

export interface SearchUsersBody {
    @minLength(1)
    term: string;
}

export interface UserResponse {
    id: i64;
    name: string;
}

export interface ApiError {
    code: string;
    message: string;
}

export interface UserValidationError {
    message: string;
}

export interface NotFoundError {
    code: string;
    message: string;
}

export interface UserPageParams {
    page: u64;
}

export interface UserPageResponse {
    readonly items: User[];
    readonly total: u64;
    readonly pageSize: u64;
    readonly page?: u64;
    readonly totalPages?: u64;
    readonly nextCursor?: string;
    readonly hasMore?: boolean;
}
```

### `src/routes/user.tyr`

```tyrus
import { HttpRouter } from "tyrus:http";

import {
    createUserController,
    deleteUserController,
    getUserController,
    listUsersController,
    paginatedUsersController,
    patchUserController,
    replaceUserController,
    searchUsersController,
} from "#/controllers/user";
import {
    ApiError,
    CreateUserBody,
    NotFoundError,
    PatchUserBody,
    ReplaceUserBody,
    SearchUsersBody,
    UserPageParams,
    UserPageResponse,
    UserParams,
    UserResponse,
    UserValidationError,
} from "#/schemas/user";

export const userRoutes = (router: HttpRouter): void => {
    router.get(
        "/users",
        {
            tags: ["user"],
            response: { 200: UserResponse[], 500: ApiError },
            handler: listUsersController,
        },
    );

    router.get(
        "/users/paginated/:page",
        {
            tags: ["user"],
            params: UserPageParams,
            response: { 200: UserPageResponse, 500: ApiError },
            handler: paginatedUsersController,
        },
    );

    router.get(
        "/users/:id",
        {
            tags: ["user"],
            params: UserParams,
            response: {
                200: UserResponse,
                400: UserValidationError,
                404: NotFoundError,
                500: ApiError,
            },
            handler: getUserController,
        },
    );

    router.post(
        "/users",
        {
            tags: ["user"],
            body: CreateUserBody,
            response: { 201: UserResponse, 500: ApiError },
            handler: createUserController,
        },
    );

    router.delete(
        "/users/:id",
        {
            tags: ["user"],
            params: UserParams,
            response: {
                200: UserResponse,
                400: UserValidationError,
                404: NotFoundError,
                500: ApiError,
            },
            handler: deleteUserController,
        },
    );

    router.patch(
        "/users/:id",
        {
            tags: ["user"],
            params: UserParams,
            body: PatchUserBody,
            response: {
                200: UserResponse,
                400: UserValidationError,
                404: NotFoundError,
                500: ApiError,
            },
            handler: patchUserController,
        },
    );

    router.put(
        "/users/:id",
        {
            tags: ["user"],
            params: UserParams,
            body: ReplaceUserBody,
            response: {
                200: UserResponse,
                400: UserValidationError,
                404: NotFoundError,
                500: ApiError,
            },
            handler: replaceUserController,
        },
    );

    router.query(
        "/users/search",
        {
            tags: ["user"],
            body: SearchUsersBody,
            response: {
                200: UserResponse[],
                400: UserValidationError,
                500: ApiError,
            },
            handler: searchUsersController,
        },
    );
};
```

### `src/controllers/user.tyr`

```tyrus
import { DbError } from "tyrus:db";
import { HttpStatus } from "tyrus:http";

import { User } from "#/models/users";
import { UserResponse } from "#/schemas/user";
import {
    createUserService,
    deleteUserService,
    getUserByIdService,
    listUsersPageService,
    listUsersService,
    patchUserService,
    replaceUserService,
    searchUsersService,
} from "#/services/user";

export const getUserController = async (req, res) => {
    const result: Result<User?, DbError> = await getUserByIdService(req.params.id);

    match (result) {
        Ok(user) => {
            if (user) {
                res.status(HttpStatus.Ok).send({ id: user.id, name: user.name });
            } else {
                res.status(HttpStatus.NotFound).send(
                    { code: "user_not_found", message: "User not found" },
                );
            }
        }
        Err(error) => {
            printError(`${error.code}: ${error.message}`);

            res.status(HttpStatus.InternalServerError).send(
                { code: "internal_server_error", message: "Could not load user" },
            );
        }
    }
};

export const createUserController = async (req, res) => {
    const inserted: User = await createUserService(req.body) onfail  {
        printError(`${inserted.code}: ${inserted.message}`);

        res.status(HttpStatus.InternalServerError).send(
            { code: "internal_server_error", message: "Could not create user" },
        );
    };

    const response: UserResponse = { id: inserted.id, name: inserted.name };

    res.status(HttpStatus.Created).send(response);
};

export const listUsersController = async (_req, res) => {
    const result = await listUsersService();

    match (result) {
        Ok(users) => {
            res.status(HttpStatus.Ok).send(users);
        }
        Err(error) => {
            printError(`${error.code}: ${error.message}`);

            res.status(HttpStatus.InternalServerError).send(
                { code: "internal_server_error", message: "Could not load users" },
            );
        }
    }
};

export const paginatedUsersController = async (req, res) => {
    const page = await listUsersPageService(req.params.page) onfail  {
        printError(`${page.code}: ${page.message}`);

        res.status(HttpStatus.InternalServerError).send(
            { code: "internal_server_error", message: "Could not load users" },
        );

        return;
    };

    res.status(HttpStatus.Ok).send(page);
};

export const deleteUserController = async (req, res) => {
    const deleted = await deleteUserService(req.params.id) onfail  {
        printError(`${deleted.code}: ${deleted.message}`);

        res.status(HttpStatus.InternalServerError).send(
            { code: "internal_server_error", message: "Could not delete user" },
        );
    };

    if (deleted) {
        res.status(HttpStatus.Ok).send({ id: deleted.id, name: deleted.name });

        return;
    }

    res.status(HttpStatus.NotFound).send(
        { code: "user_not_found", message: "User not found" },
    );
};

export const patchUserController = async (req, res) => {
    const updated = await patchUserService(req.params.id, req.body.name) onfail  {
        printError(`${updated.code}: ${updated.message}`);

        res.status(HttpStatus.InternalServerError).send(
            { code: "internal_server_error", message: "Could not update user" },
        );
    };

    if (updated) {
        res.status(HttpStatus.Ok).send({ id: req.params.id, name: req.body.name });

        return;
    }

    res.status(HttpStatus.NotFound).send(
        { code: "user_not_found", message: "User not found" },
    );
};

export const replaceUserController = async (req, res) => {
    const updated = await replaceUserService(req.params.id, req.body.name) onfail  {
        printError(`${updated.code}: ${updated.message}`);

        res.status(HttpStatus.InternalServerError).send(
            { code: "internal_server_error", message: "Could not replace user" },
        );
    };

    if (updated) {
        res.status(HttpStatus.Ok).send({ id: req.params.id, name: req.body.name });

        return;
    }

    res.status(HttpStatus.NotFound).send(
        { code: "user_not_found", message: "User not found" },
    );
};

export const searchUsersController = async (req, res) => {
    const users = await searchUsersService(req.body.term) onfail  {
        printError(`${users.code}: ${users.message}`);

        res.status(HttpStatus.InternalServerError).send(
            { code: "internal_server_error", message: "Could not search users" },
        );
    };

    res.status(HttpStatus.Ok).send(users);
};
```

### `src/services/user.tyr`

```tyrus
import { DbError } from "tyrus:db";

import { User } from "#/models/users";
import {
    createUserRepository,
    deleteUserRepository,
    findUserByIdRepository,
    findUsersPageRepository,
    findUsersRepository,
    patchUserRepository,
    replaceUserRepository,
    searchUsersRepository,
} from "#/repositories/user";
import {
    CreateUserBody,
    UserPageResponse,
    UserResponse,
} from "#/schemas/user";

export const listUsersService = (): Promise<Result<UserResponse[], DbError>> => {
    return findUsersRepository();
};

export const getUserByIdService = (id: i64): Promise<Result<User?, DbError>> => {
    return findUserByIdRepository(id);
};

export const createUserService = (body: CreateUserBody): Promise<Result<User, DbError>> => {
    return createUserRepository(body);
};

export const listUsersPageService = (page: u64): Promise<Result<UserPageResponse, DbError>> => {
    return findUsersPageRepository(page);
};

export const deleteUserService = (id: i64): Promise<Result<User?, DbError>> => {
    return deleteUserRepository(id);
};

export const patchUserService = (id: i64, name: string): Promise<Result<User?, DbError>> => {
    return patchUserRepository(id, name);
};

export const replaceUserService = (id: i64, name: string): Promise<Result<User?, DbError>> => {
    return replaceUserRepository(id, name);
};

export const searchUsersService = (term: string): Promise<Result<UserResponse[], DbError>> => {
    return searchUsersRepository(term);
};
```

### `src/repositories/user.tyr`

```tyrus
import { DbError, db } from "tyrus:db";

import { User } from "#/models/users";
import {
    CreateUserBody,
    UserPageResponse,
    UserResponse,
} from "#/schemas/user";

export const findUsersRepository = (): Promise<Result<UserResponse[], DbError>> => {
    return db.query<UserResponse>("SELECT id, name FROM users ORDER BY id", []);
};

export const findUserByIdRepository = (id: i64): Promise<Result<User?, DbError>> => {
    return User.findOne({ id });
};

export const createUserRepository = (body: CreateUserBody): Promise<Result<User, DbError>> => {
    return User.insertOne({ name: body.name });
};

export const findUsersPageRepository = (page: u64): Promise<Result<UserPageResponse, DbError>> => {
    return User.findMany({ }, { paginate: true, pageSize: 1, page });
};

export const deleteUserRepository = async (id: i64): Promise<Result<User?, DbError>> => {
    const deleted = await User.deleteOne({ id }) onfail  {
        return Err(deleted);
    };

    return Ok(deleted);
};

export const patchUserRepository = async (id: i64, name: string): Promise<Result<User?, DbError>> => {
    const updated = await User.updateOne({ id }, { name }) onfail  {
        return Err(updated);
    };

    return Ok(updated);
};

export const replaceUserRepository = async (id: i64, name: string): Promise<Result<User?, DbError>> => {
    const updated = await User.updateOne({ id }, { name }) onfail  {
        return Err(updated);
    };

    return Ok(updated);
};

export const searchUsersRepository = (term: string): Promise<Result<UserResponse[], DbError>> => {
    return db.query<UserResponse>(
        "SELECT id, name FROM users WHERE name LIKE ? ORDER BY id",
        [`%${term}%`],
    );
};
```
