```tyrus
const values: i32[] = [2, 3, 4];
const totals: i32[] = values.scan(0, (total, value) => total + value); // [0, 2, 5, 9]
print(totals);
```
