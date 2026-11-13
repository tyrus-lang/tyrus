```tyrus
const ll = LinkedList<i32>();

ll.pushBack(10);
ll.pushBack(20);

const f: i32? = ll.front();
if (f) {
    print(f);  // 10
}

const b: i32? = ll.back();
if (b) {
    print(b);  // 20
}

```
