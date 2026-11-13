```tyrus
function loadUser(): Result<User, string> {
    return Ok(fetchUser());
}

function render(): Result<void, string> {
    const user = loadUser()?;
    print(user.name);
    
    return Ok();
}

function validate(active: boolean): Result<string, boolean> {
    if (active) {
        return Ok("ready");
    }
    return Err(false);
}

function printValidation(result: Result<string, boolean>): void {
    match (result) {
        Ok(value) => print(value);
        Err(error) => print(error);
    }
}
```
