```tyrus
const st = SegmentTree([1, 3, 5, 7, 9, 11]);

st.update(0, 2);
st.update(5, 4);

print(st.query(0, 6));  // 2 + 3 + 5 + 7 + 9 + 4
```
