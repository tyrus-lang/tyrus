```tyrus
function transfer(ok: boolean): Result<void, string> {
    defer print("cleanup"); // cleanup
    errdefer print("rollback"); // rollback

    if (!ok) {
        return Err("transfer failed");
    }
    
    return Ok();
}

const main = (): void => {
    match (transfer(false)) {
        Ok() => print("transfer complete"); // transfer complete
        Err(error) => printError(error); // transfer failed
    }
};
```
