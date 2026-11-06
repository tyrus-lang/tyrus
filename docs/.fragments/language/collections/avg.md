```tyrus
const values: i32[] = [2, 3, 4];
const average = values.avg(); // inferred as f64?
print(average ?? 0.0); // 3.0

const empty: i32[] = [];
print(empty.avg() ?? 0.0); // 0.0
```
