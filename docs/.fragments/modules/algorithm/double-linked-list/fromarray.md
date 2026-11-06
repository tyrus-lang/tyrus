```tyrus
const vals: i32[] = [7, 8, 9];
const dll = DoubleLinkedList.fromArray(vals);

print(dll.len());  // 3

const f: i32? = dll.front();
if (f) {
    print(f);  // 7
}

const b: i32? = dll.back();
if (b) {
    print(b);  // 9
}
```
