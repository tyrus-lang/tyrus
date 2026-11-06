```tyrus
const values: i32[] = [1, 2, 3, 4, 5];
const oddAndEven: i32[] = values.uniqueBy(value => value % 2); // [1, 2]
const oddAndEvenImplicit: i32[] = values.uniqueBy(value % 2); // [1, 2]
print(oddAndEven);
```
