```tyrus
const bs = BitSet(100);
bs.set(3);
bs.set(5);
bs.set(7);

print(bs.complement().count());  // 97
print(bs.complement().get(1));   // true

```
