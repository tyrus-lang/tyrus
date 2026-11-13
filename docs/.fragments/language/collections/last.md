```tyrus
const values: i32[] = [10, 20, 30];
const lastValue = values.last(); // inferred as i32?
const explicitLast: Optional<i32> = values.last();

const lastOrDefault: i32 = lastValue ? lastValue : 0;
print(lastOrDefault);

const empty: i32[] = [];
print(empty.last() ?? -1); // -1
```
