```tyrus
const st = SegmentTree([10, 20, 30]);

st.update(1, -5);

print(st.query(0, 2));  // 10 + (-5)
print(st.query(0, 3));  // 10 + (-5) + 30
```
