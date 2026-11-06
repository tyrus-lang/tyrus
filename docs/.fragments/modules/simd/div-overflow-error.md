```tyrus
import { simdDiv, SimdError } from "tyrus:simd";

const left: i8[] = [-128; 1];
const right: i8[] = [-1; 1];
const result: Result<i8[], SimdError> = simdDiv(left, right);

match (result) {
    Ok(values) => { print(values); }
    Err(error) => { printError(error.code, error.message); }
}
```

```text
ERR_SIMD_INTEGER_OVERFLOW simdDiv cannot represent the signed minimum divided by -1
```
