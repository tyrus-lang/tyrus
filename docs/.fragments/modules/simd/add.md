```tyrus
import { simdAdd, SimdError } from "tyrus:simd";

const left: f32[] = [1.0, 2.0, 3.0, 4.0, 5.0];
const right: f32[] = [10.0, 20.0, 30.0, 40.0, 50.0];
const result: Result<f32[], SimdError> = simdAdd(left, right);

match (result) {
    Ok(values) => {
        print(values); // [11.0, 22.0, 33.0, 44.0, 55.0]
    }
    Err(error) => {
        printError(error.code, error.message);
    }
}
```

Fixed-size arrays can be passed directly as well. Their length is known at compile time, and no dynamic input array is allocated:

```tyrus
const fixedLeft: [i32; 5] = [1, 2, 3, 4, 5];
const fixedRight: [i32; 5] = [10, 20, 30, 40, 50];
const fixedResult: Result<i32[], SimdError> = simdAdd(fixedLeft, fixedRight);

match (fixedResult) {
    Ok(values) => {
        print(values); // [11, 22, 33, 44, 55]
    }
    Err(error) => {
        printError(error.code, error.message);
    }
}
```

The same operation accepts integer arrays. Signed and unsigned integer addition wraps at the element width by default, while the declared element type is still checked for literal range at compile time:

```tyrus
const unsignedLeft: u8[] = [255; 5];
const unsignedRight: u8[] = [1; 5];
const unsignedResult: Result<u8[], SimdError> = simdAdd(unsignedLeft, unsignedRight);

match (unsignedResult) {
    Ok(values) => {
        print(values); // [0, 0, 0, 0, 0]
    }
    Err(error) => {
        printError(error.code, error.message);
    }
}
```

Signed integer values wrap too. Here `120 + 120` is reduced to the `i8` range and produces `-16` without a runtime overflow error:

```tyrus
import { simdAdd, SimdError } from "tyrus:simd";

const signedLeft: i8[] = [120; 5];
const signedRight: i8[] = [120; 5];
const signedResult: Result<i8[], SimdError> = simdAdd(signedLeft, signedRight);

match (signedResult) {
    Ok(values) => {
        print(values); // [-16, -16, -16, -16, -16]
    }
    Err(error) => {
        printError(error.code, error.message);
    }
}
```

