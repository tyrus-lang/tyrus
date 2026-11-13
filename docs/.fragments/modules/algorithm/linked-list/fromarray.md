```tyrus
const vals: i32[] = [7, 8, 9];
const ll = LinkedList.fromArray(vals);

print(ll.len());  // 3

const f: i32? = ll.front();
if (f) {
    print(f);  // 7
}

const b: i32? = ll.back();
if (b) {
    print(b);  // 9
}
```
