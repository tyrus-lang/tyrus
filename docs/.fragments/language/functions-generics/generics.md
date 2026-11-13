```tyrus
const identity = <T>(value: T): T => value;
const first = <T extends string>(value: T): T => value;

const number: i32 = identity<i32>(42);
const message: string = identity("tyrus");
const constrained: string = first("native");
```
