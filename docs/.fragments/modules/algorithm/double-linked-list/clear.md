```tyrus
const dll = DoubleLinkedList<i32>();

dll.pushBack(1);
dll.pushBack(2);
dll.pushBack(3);

print(dll.len());  // 3

dll.clear();

print(dll.isEmpty());  // true
print(dll.len());      // 0

```
