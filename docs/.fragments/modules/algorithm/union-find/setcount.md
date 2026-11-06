```tyrus
const uf = UnionFind<u32>(5);

print(uf.setCount());  // 5

uf.union(0, 1);
uf.union(2, 3);

print(uf.setCount());  // 3

```
