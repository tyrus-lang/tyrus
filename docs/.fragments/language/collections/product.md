```tyrus
const values: i32[] = [2, 3, 4];
const product = values.product(); // inferred as i32
print(product); // 24

const empty: i32[] = [];
print(empty.product()); // 1
```
