```tyrus
const uf = UnionFind<u32>(5);

print(uf.union(0, 1));  // true
print(uf.union(2, 3));  // true
print(uf.union(0, 1));  // false

```
