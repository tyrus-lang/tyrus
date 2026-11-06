```tyrus
import { ChildProcess, ProcessError, exec } from "tyrus:process";

async function run(): Promise<Result<i32, ProcessError>> {
    const child: ChildProcess = exec("tool", ["--version"])?;
    await child.stdin.close();
    return await child.wait();
}
```

::: danger error
```tyrus
const child: ChildProcess = exec("tool", "--version"); // [!code error]
```
:::
