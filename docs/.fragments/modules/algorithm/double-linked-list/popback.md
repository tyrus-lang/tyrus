```tyrus
const dll = DoubleLinkedList<i32>();

dll.pushBack(1);
dll.pushBack(2);
dll.pushBack(3);

const p: i32? = dll.popBack();
if (p) {
    print(p);  // 3
}

print(dll.len());  // 2

```
