```tyrus
import { AlgorithmError, spfa } from "tyrus:algorithm";

const showPaths = (): Result<void, AlgorithmError> => {
    const result = spfa([[[1, 2]], [[2, -1]], []], 0)?;
    print(result[0][2]); // 1
    return Ok();
};

match (showPaths()) {
    Ok(_) => {}
    Err(error) => { printError(error.code); // ERR_ALGORITHM_NEGATIVE_CYCLE
    }
}
```
