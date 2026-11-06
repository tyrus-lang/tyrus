```tyrus
enum Role: string {
    Admin = "admin",
    User = "user",
}

const allowedRoles: Role[] = [Role.Admin, Role.User];

const authorize = (@in(allowedRoles) role: Role): void => {
    print(role); // Role.Admin or Role.User
};
```
