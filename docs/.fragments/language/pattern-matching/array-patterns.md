```tyrus
function inspect(values: i32[]): void {
    match (values) {
        [] => { print("empty"); }
        [value] => { print(value); }
        [head, ...tail] => { print(head, tail.length); }
    }
}
```
