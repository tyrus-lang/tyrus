```text
error[async-missing-promise-return]: async function must return `Promise<T>`, found `i32`
at /src/main.tyr:1:23
async function bad(): i32 {
                      ^^^
help: change the return type to `Promise<i32>`
```
