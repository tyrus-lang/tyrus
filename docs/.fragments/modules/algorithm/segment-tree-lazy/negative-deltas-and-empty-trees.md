```tyrus
const st = SegmentTreeLazy([10, 20, 30]);

st.updateRange(1, 3, -5);

print(st.query(0, 3));  // 50

const empty: i32[] = [];
const emptyTree = SegmentTreeLazy(empty);
print(emptyTree.isEmpty());  // true
print(emptyTree.len());      // 0
```
