```tyrus
import { simdDiv, SimdError } from "tyrus:simd";

const main = (): void => {
    const numerators: f32[] = [1.0, -1.0, 0.0];
    const denominators: f32[] = [0.0; 3];
    const result: Result<f32[], SimdError> = simdDiv(numerators, denominators);

    match (result) {
        Ok(values) => {
            print(values); // [Infinity, -Infinity, NaN]
        }
        Err(error) => {
            printError(error.code, error.message);
        }
    }
};
```
