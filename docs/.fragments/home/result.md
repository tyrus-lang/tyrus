```tyrus
function lookupName(found: boolean): Result<string, string> {
    if (found) {
        return Ok("Ada");
    }
    
    return Err("user not found");
}

const main = (): void => {
    match (lookupName(false)) {
        Ok(name) => print(name);
        Err(message) => printError(message); // user not found
    }
};
```
