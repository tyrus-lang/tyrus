```tyrus
const dq = Deque<i32>();

dq.pushBack(10);
dq.pushBack(20);

const f: i32? = dq.peekFront();
if (f) {
    print(f);  // 10
}

const b: i32? = dq.peekBack();
if (b) {
    print(b);  // 20
}

print(dq.len());  // 2

```
