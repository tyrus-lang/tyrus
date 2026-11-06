```tyrus
const dll = DoubleLinkedList<i32>();

dll.pushBack(10);
dll.pushBack(20);

const f: i32? = dll.front();
if (f) {
    print(f);  // 10
}

const b: i32? = dll.back();
if (b) {
    print(b);  // 20
}

```
