```tyrus
const values: i32[] = [1, 2, 3, 4];
const [evenValues, oddValues] = values.partition(value % 2 === 0);
print(evenValues); // [2, 4]
print(oddValues); // [1, 3]
print(values); // [1, 2, 3, 4]
```
