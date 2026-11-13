```tyrus
const bh = BinaryHeap<i32>();

bh.push(10);
bh.push(3);

const top: i32? = bh.peek();
if (top) {
    print(top);  // 10
}

```
