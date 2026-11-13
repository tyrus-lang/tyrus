```tyrus
const bit = FenwickTree(4);

bit.add(0, 1);
bit.add(1, 2);
bit.add(2, 3);
bit.add(3, 4);

print(bit.sum(0));  // 0
print(bit.sum(1));  // 1
print(bit.sum(3));  // 1 + 2 + 3
print(bit.sum(4));  // 1 + 2 + 3 + 4
```
