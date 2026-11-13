```tyrus
function render(): Result<void, string> {
    const result = loadUser() onfail {
        printError(result);
    };

    print(result.name);
    return Ok();
}
```
