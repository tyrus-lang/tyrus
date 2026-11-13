```tyrus
const bs = BitSet(100);

bs.set(5);
bs.set(10, true);
bs.set(3, false);

print(bs.get(5));   // true
print(bs.get(3));   // false
print(bs.get(10));  // true

```
