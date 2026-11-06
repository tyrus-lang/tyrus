```tyrus
import { simdDiv, SimdError } from "tyrus:simd";

const left: i32[] = [100, 81, 72, 55];
const right: i32[] = [10, 0, 8, 5];
const result: Result<i32[], SimdError> = simdDiv(left, right);

match (result) {
    Ok(values) => { print(values); }
    Err(error) => { printError(error.code, error.message); }
}
```

```text
ERR_SIMD_DIVISION_BY_ZERO simdDiv cannot divide an integer lane by zero
```
