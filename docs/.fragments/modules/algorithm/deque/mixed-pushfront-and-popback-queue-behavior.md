```tyrus
const dq = Deque<i32>();

dq.pushBack(1);
dq.pushBack(2);
dq.pushFront(0);

const p1: i32? = dq.popBack();
if (p1) {
    print(p1);  // 2
}

const p2: i32? = dq.popFront();
if (p2) {
    print(p2);  // 0
}

const p3: i32? = dq.popBack();
if (p3) {
    print(p3);  // 1
}

print(dq.isEmpty());  // true

```
