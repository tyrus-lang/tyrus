```tyrus
function process(value: string | number): void {
    if (value is string) {
        print(value.length); // string length
    } else {
        print(value + 1); // number
    }
}
```
