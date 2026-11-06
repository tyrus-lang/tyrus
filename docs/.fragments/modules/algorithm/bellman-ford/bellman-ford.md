```tyrus
import { AlgorithmError, bellmanFord } from "tyrus:algorithm";

const showPaths = (): Result<void, AlgorithmError> => {
    const graph: [usize, i32][][] = [[[1, 2]], [[2, -1]], []];
    const result = bellmanFord(graph, 0)?;
    print(result[0]); // [0, 2, 1]
    return Ok();
};

match (showPaths()) {
    Ok(_) => {}
    Err(error) => { printError(error.code); // ERR_ALGORITHM_NEGATIVE_CYCLE
    }
}
```
