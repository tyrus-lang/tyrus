```tyrus
function describeValue(value: string | i32): void {
    if (typeof value === "string") {
        print(value.length); // string length
    } else if (typeof value === "i32") {
        print(value + 1); // value + 1
    }
}
```
