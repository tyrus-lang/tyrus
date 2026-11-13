```tyrus
import { ChildProcess, ProcessError, exec } from "tyrus:process";
import { ReadableStream, WritableStream } from "tyrus:stream";

async function run(command: string, args?: string[]): Promise<Result<i32, ProcessError>> {
    const child: ChildProcess = exec(command, args)?;
    const input: WritableStream<u8[]> = child.stdin;
    const output: ReadableStream<u8[]> = child.stdout;
    const diagnostics: ReadableStream<u8[]> = child.stderr;

    await input.close();
    return await child.wait();
}
```
