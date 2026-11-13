```tyrus
const indexes: usize[] = [3, 5, 10];
const bs = BitSet.fromArray(indexes);

print(bs.count());  // 3
print(bs.get(10));  // true
print(bs.get(4));   // false

```
