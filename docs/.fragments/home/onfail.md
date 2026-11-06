```tyrus
function lookupName(found: boolean): Result<string, string> {
    if (found) {
        return Ok("Ada");
    }

    return Err("user not found");
}

function showName(found: boolean): void {
    const name = lookupName(found) onfail {
        printError(name); // user not found

        return;
    };
    
    print(name); // Ada
}

const main = () => showName(true);
```
