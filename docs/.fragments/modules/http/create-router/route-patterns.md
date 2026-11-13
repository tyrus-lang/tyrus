```tyrus
export const userRoutes = (router: HttpRouter): void => {
    router.get("/users", {
        response: { 200: UserResponse[], 400: ValidationError, 500: ApiError },
        handler: listUsersController,
    });

    router.post("/users", {
        body: CreateUserBody,
        response: { 201: UserResponse, 409: ConflictError, 500: ApiError },
        handler: createUserController,
    });

    router.patch("/users/:id", {
        params: UserParams,
        body: PatchUserBody,
        response: { 200: UserResponse, 400: ValidationError, 404: NotFoundError, 409: ConflictError, 500: ApiError },
        handler: patchUserController,
    });

    router.put("/users/:id", {
        params: UserParams,
        body: ReplaceUserBody,
        response: { 200: UserResponse, 400: ValidationError, 404: NotFoundError, 409: ConflictError, 500: ApiError },
        handler: replaceUserController,
    });

    router.query("/users/search", {
        body: SearchUsersBody,
        response: { 200: UserResponse[], 400: ValidationError, 403: ForbiddenError, 500: ApiError },
        handler: searchUsersController,
    });

    router.delete("/users/:id", {
        params: UserParams,
        response: { 204: void, 400: ValidationError, 404: NotFoundError, 500: ApiError },
        handler: deleteUserController,
    });
};
```
