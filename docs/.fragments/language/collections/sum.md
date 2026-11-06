```tyrus
const values: i32[] = [2, 3, 4];
const total = values.sum(); // inferred as i32
print(total); // 9

const empty: i32[] = [];
print(empty.sum()); // 0
```
