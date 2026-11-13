```tyrus
const bs = BitSet(100);

print(bs.count());     // 0
print(bs.isEmpty());   // true

bs.set(1);
bs.set(2);

print(bs.count());    // 2
print(bs.isEmpty());  // false

```
