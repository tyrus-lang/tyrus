```tyrus
const dq = Deque<i32>();

dq.pushBack(1);
dq.pushBack(2);
dq.pushBack(3);

const p1: i32? = dq.popBack();
if (p1) {
    print(p1);  // 3
}

const p2: i32? = dq.popBack();
if (p2) {
    print(p2);  // 2
}

const p3: i32? = dq.popBack();
if (p3) {
    print(p3);  // 1
}

const p4: i32? = dq.popBack();
print(p4 ?? -1);  // -1

```
