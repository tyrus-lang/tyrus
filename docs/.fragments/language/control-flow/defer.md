```tyrus
function process(): void {
    defer print("finished");
    defer { print("released"); }
    print("working");
}

function transfer(ok: boolean): Result<void, i32> {
    defer print("always");
    errdefer { print("rollback"); }
    if (!ok) { return Err(1); }
    return Ok();
}
```
