```tyrus
import { simdDiv, SimdError } from "tyrus:simd";

const left: i32[] = [100, 81, 72, 55, 42];
const right: i32[] = [10, 9, 8, 5, 6];
const result: Result<i32[], SimdError> = simdDiv(left, right);

match (result) {
    Ok(values) => {
        print(values); // [10, 9, 9, 11, 7]
    }
    Err(error) => {
        printError(error.code, error.message);
    }
}
```

Floating-point division follows IEEE behavior. Dividing nonzero values by `0.0` produces infinity, and `0.0 / 0.0` produces NaN; these results are returned in `Ok`.
