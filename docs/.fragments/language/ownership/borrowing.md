```tyrus
const original: string = "tyrus";
const borrowed: &string = &original;
print(borrowed); // "tyrus"

let counter: i32 = 0;
const mutable: &mut i32 = &mut counter;
```
