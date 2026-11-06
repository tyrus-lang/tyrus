```tyrus
import { UnionFind, BinaryHeap } from "tyrus:algorithm";

const uf: UnionFind = UnionFind.new(10);
uf.union(0, 1);
uf.union(2, 3);
print(uf.connected(0, 1));
print("assert:", true);
print(uf.connected(0, 2));
print("assert:", false);
uf.dispose();
```
