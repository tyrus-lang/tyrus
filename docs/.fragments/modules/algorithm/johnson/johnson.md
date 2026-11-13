```tyrus
import { AlgorithmError, johnson } from "tyrus:algorithm";

const showDistances = (): Result<void, AlgorithmError> => {
    const distances = johnson([[[1, 4], [2, 2]], [[3, 5]], [[3, 1]], []])?;
    print(distances[0][3] ?? -1); // 3
    return Ok();
};

match (showDistances()) {
    Ok(_) => {}
    Err(error) => { printError(error.code); // ERR_ALGORITHM_NEGATIVE_CYCLE
    }
}
```
