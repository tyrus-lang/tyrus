```tyrus
const bh = BinomialHeap<i32>();

bh.push(10);
bh.push(3);
bh.push(7);

print(bh.len());  // 3

bh.clear();

print(bh.isEmpty());  // true
print(bh.len());      // 0

```
