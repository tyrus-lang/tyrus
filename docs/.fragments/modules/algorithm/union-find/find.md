```tyrus
const uf = UnionFind<u32>(5);

print(uf.find(0));  // 0
print(uf.find(3));  // 3

uf.union(0, 1);
uf.union(1, 2);

print(uf.find(2));  // 0

```
