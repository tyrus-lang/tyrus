```tyrus
interface User { name: string; }

const result: Result<User, JsonError> = User.fromJson("{\"name\":\"Ada\"}");
match (result) {
    Ok(user) => {
        print(user.name); // Ada
    }
    Err(error) => {
        printError(error.code);
    }
}
```
