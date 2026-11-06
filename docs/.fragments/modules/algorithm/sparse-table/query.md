```tyrus
const st = SparseTable([7, 2, 9, 1, 5, 8, 3]);

print(st.query(0, 7));  // minimum of all seven values
print(st.query(0, 3));  // min(7, 2, 9)
print(st.query(2, 5));  // min(9, 1, 5)
print(st.query(1, 2));  // 2
```
