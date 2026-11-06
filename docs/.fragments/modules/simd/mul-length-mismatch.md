```tyrus
import { simdMul, SimdError } from "tyrus:simd";

const left: i32[] = [1, 2];
const right: i32[] = [3, 4, 5];
const result: Result<i32[], SimdError> = simdMul(left, right);

match (result) {
    Ok(_) => {}
    Err(error) => { printError(error.code, error.message); }
}
```

```text
ERR_SIMD_LENGTH_MISMATCH simdMul requires arrays of equal length
```
