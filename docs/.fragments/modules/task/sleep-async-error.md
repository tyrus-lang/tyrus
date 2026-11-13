```tyrus
import { sleepAsync } from "tyrus:task";

function main(): void {
    await sleepAsync(25); // [!code error]
}
```

```text
error[async-await-outside-async]: `await` can only be used inside an async function
at /src/main.tyr:4:5
    await sleepAsync(25);
    ^^^^^
help: move this `await` into an `async` function or mark its enclosing function `async`
```
