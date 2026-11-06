```tyrus
const a = BitSet(100);
a.set(1);
a.set(3);

const b = BitSet(100);
b.set(1);
b.set(3);
b.set(5);

print(a.isSubset(b));    // true
print(b.isSuperset(a));  // true
print(b.isSubset(a));    // false

```
