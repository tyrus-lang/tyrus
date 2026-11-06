```tyrus
const bit = FenwickTree(3);

bit.add(0, 10);
bit.add(1, 20);
bit.add(2, 30);

bit.add(1, -5);

print(bit.sum(2));  // 10 + 15
print(bit.sum(3));  // 10 + 15 + 30
```
