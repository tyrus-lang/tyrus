```tyrus
const st = SegmentTree([1, 3, 5, 7, 9, 11]);

print(st.query(0, 6));  // 1 + 3 + 5 + 7 + 9 + 11
print(st.query(0, 3));  // 1 + 3 + 5
print(st.query(2, 5));  // 5 + 7 + 9
print(st.query(1, 2));  // 3
```
