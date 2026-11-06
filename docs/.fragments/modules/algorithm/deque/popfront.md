```tyrus
const dq = Deque<i32>();

dq.pushBack(1);
dq.pushBack(2);
dq.pushBack(3);

const p1: i32? = dq.popFront();
if (p1) {
    print(p1);  // 1
}

const p2: i32? = dq.popFront();
if (p2) {
    print(p2);  // 2
}

const p3: i32? = dq.popFront();
if (p3) {
    print(p3);  // 3
}

const p4: i32? = dq.popFront();
print(p4 ?? -1);  // -1

```
