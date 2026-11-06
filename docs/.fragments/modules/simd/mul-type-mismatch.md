```tyrus
import { simdMul } from "tyrus:simd";

const left: i32[] = [1, 2];
const right: i64[] = [3, 4];
const result = simdMul(left, right); // [!code error]
```

```text
error[E0308]: type mismatch for argument 2 of `simdMul`: expected `i32[]`, found `i64[]`
at /src/main.tyr:5:16
const result = simdMul(left, right);
               ^^^^^^^
```
