```tyrus
import { swap } from "tyrus:memory";

function main(): void {
    let left: i32 = 4;
    let right: i32 = 9;
    print(left); // 4
    print(right); // 9
    swap(left, right);
    print(left); // 9
    print(right); // 4

    let first: string = "first";
    let second: string = "second";
    swap(first, second);
    print(first); // second
    print(second); // first
}
```
