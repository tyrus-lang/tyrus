```tyrus
import { simdSub, SimdError } from "tyrus:simd";

const left: i32[] = [20, 30, 40, 50, 60];
const right: i32[] = [3, 4, 5, 6, 7];
const result: Result<i32[], SimdError> = simdSub(left, right);

match (result) {
    Ok(values) => {
        print(values); // [17, 26, 35, 44, 53]
    }
    Err(error) => {
        printError(error.code, error.message);
    }
}
```
