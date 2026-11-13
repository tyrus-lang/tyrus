```tyrus
const a = BitSet(100);
a.set(1);
a.set(3);
a.set(5);

const b = BitSet(100);
b.set(3);
b.set(5);
b.set(7);

print(a.union(b).count());        // 4
print(a.intersection(b).count()); // 2
print(a.difference(b).count());   // 1

```
