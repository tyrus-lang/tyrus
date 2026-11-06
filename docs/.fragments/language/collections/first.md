```tyrus
const values: i32[] = [10, 20, 30];
const firstValue = values.first(); // inferred as i32?
const explicitFirst: Optional<i32> = values.first();

if (firstValue) {
    print(firstValue + 1); // 11
}

const firstOrDefault: i32 = firstValue ?? 0;
print(firstOrDefault);

const empty: i32[] = [];
print(empty.first() ?? -1); // -1
```
