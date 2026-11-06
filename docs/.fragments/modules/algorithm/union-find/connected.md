```tyrus
const uf = UnionFind<u32>(5);

uf.union(0, 1);
uf.union(2, 3);

print(uf.connected(0, 1));  // true
print(uf.connected(0, 3));  // false
print(uf.connected(2, 3));  // true

```
