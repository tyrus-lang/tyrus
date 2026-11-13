```tyrus
const ll = LinkedList<i32>();

ll.pushBack(1);
ll.pushBack(2);
ll.pushBack(3);

const p1: i32? = ll.popFront();
if (p1) {
    print(p1);  // 1
}

const p2: i32? = ll.popFront();
if (p2) {
    print(p2);  // 2
}

print(ll.len());  // 1

```
