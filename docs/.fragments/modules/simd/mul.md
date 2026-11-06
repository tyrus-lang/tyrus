```tyrus
import { simdMul, SimdError } from "tyrus:simd";

const left: f32[] = [2.0, 3.0, 4.0, 5.0, 6.0];
const right: f32[] = [3.0, 4.0, 5.0, 6.0, 7.0];
const result: Result<f32[], SimdError> = simdMul(left, right);

match (result) {
    Ok(values) => {
        print(values); // [6.0, 12.0, 20.0, 30.0, 42.0]
    }
    Err(error) => {
        printError(error.code, error.message);
    }
}
```
