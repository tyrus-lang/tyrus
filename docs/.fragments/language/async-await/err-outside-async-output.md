```text
error[async-await-outside-async]: `await` can only be used inside an async function
at /src/main.tyr:6:24
    const value: i32 = await getData();
                       ^^^^^
help: move this `await` into an `async` function or mark its enclosing function `async`
```
