```tyrus
import { replace } from "tyrus:memory";

function main(): void {
    let value: i32 = 4;
    const previous: i32 = replace(value, 12);
    print(value); // 12
    print(previous); // 4

    let source: string = "new value";
    let destination: string = "old value";
    const oldText: string = replace(destination, source);
    print(destination); // new value
    print(oldText); // old value
}
```
