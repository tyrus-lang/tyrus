```tyrus
const ll = LinkedList<i32>();

ll.pushBack(1);
ll.pushBack(2);
ll.pushBack(3);

const p: i32? = ll.popBack();
if (p) {
    print(p);  // 3
}

print(ll.len());  // 2

```
