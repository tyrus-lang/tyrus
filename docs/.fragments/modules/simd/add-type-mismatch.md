```tyrus
import { simdAdd } from "tyrus:simd";

const left: i32[] = [1, 2];
const right: i64[] = [3, 4];
const result = simdAdd(left, right); // [!code error]
```

```text
error[E0308]: type mismatch for argument 2 of `simdAdd`: expected `i32[]`, found `i64[]`
at /src/main.tyr:5:16
const result = simdAdd(left, right);
               ^^^^^^^
```
