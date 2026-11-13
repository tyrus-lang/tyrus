```tyrus
function handle(value: string | number): void {
    if (value not is string) {
        print(value + 1); // number
    } else {
        print(value.length); // string
    }
}
```
