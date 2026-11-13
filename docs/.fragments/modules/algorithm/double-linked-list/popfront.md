```tyrus
const dll = DoubleLinkedList<i32>();

dll.pushBack(1);
dll.pushBack(2);
dll.pushBack(3);

const p1: i32? = dll.popFront();
if (p1) {
    print(p1);  // 1
}

const p2: i32? = dll.popFront();
if (p2) {
    print(p2);  // 2
}

print(dll.len());  // 1

```
