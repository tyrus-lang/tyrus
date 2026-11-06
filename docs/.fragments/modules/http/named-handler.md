```tyrus
// schemas/users.tyr
export interface UserParams {
    id: string;
}

export interface UserResponse {
    id: string;
}

export interface ApiError {
    code: string;
    message: string;
}
```

```tyrus
// controllers/users.tyr
import { HttpStatus } from "tyrus:http";

export const getUser = async (req, res) => {
    if (req.params.id === "user-404") {
        res.status(HttpStatus.NotFound).send({ code: "user_not_found", message: "User not found" });
    } else {
        res.status(HttpStatus.Ok).send({ id: req.params.id });
    }
};
```

```tyrus
// routes/users.tyr
import { createRouter, HttpRouter } from "tyrus:http";
import { ApiError, UserParams, UserResponse } from "#/schemas/users";
import { getUser } from "#/controllers/users";

const app: HttpRouter = createRouter();

app.get("/users/:id", {
    params: UserParams,
    response: { 200: UserResponse, 404: ApiError },
    handler: getUser,
});

app.listen(3000);
```

When an imported controller leaves its two handler parameter types out, the route supplies them from `params`, `query`, `headers`, `cookies`, `body`, and `response`. The `req` fields and the `res.status(...).send(...)` body are checked against those schemas without importing their types into the controller. Explicit parameter annotations remain supported. One inferred controller cannot serve routes with different request or response contracts; declare a shared explicit contract or use a separate controller for each route.
