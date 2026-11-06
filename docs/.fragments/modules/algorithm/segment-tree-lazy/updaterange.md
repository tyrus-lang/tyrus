```tyrus
const st = SegmentTreeLazy([1, 3, 5, 7, 9, 11]);

st.updateRange(1, 4, 10);

print(st.query(0, 6));  // 66
print(st.query(1, 4));  // 45
```
