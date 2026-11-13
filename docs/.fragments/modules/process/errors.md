```tyrus
import { ChildProcess, ProcessError, exec } from "tyrus:process";

async function run(command: string): Promise<Result<i32, ProcessError>> {
    const child: ChildProcess = exec(command)?;
    return await child.wait();
}
```

::: danger error
```tyrus
const child: ChildProcess = exec(42); // [!code error]
```
:::
