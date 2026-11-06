```tyrus
const values: i32[] = [1, 2, 3, 4];
const groups: Map<i32, i32[]> = values.groupBy(value => value % 2); // { 1: [1, 3], 0: [2, 4] }
const groupsImplicit: Map<i32, i32[]> = values.groupBy(value % 2); // { 1: [1, 3], 0: [2, 4] }
print(groups.keys());
```
