```tyrus
const bh = BinaryHeap<i32>();

bh.push(10);
bh.push(3);
bh.push(7);

const p1: i32? = bh.pop();
if (p1) {
    print(p1);  // 10
}

const p2: i32? = bh.pop();
if (p2) {
    print(p2);  // 7
}

const p3: i32? = bh.pop();
if (p3) {
    print(p3);  // 3
}

const p4: i32? = bh.pop();
print(p4 ?? -1);  // -1

```
