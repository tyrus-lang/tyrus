```tyrus
interface JohnsonInfo {
    distances: i32?[][];
    reweightingCount: usize;
    bellmanFordRelaxationCount: usize;
    dijkstraExecutionCount: usize;
}
```

```tyrus
import { AlgorithmError, JohnsonInfo, johnsonInfo } from "tyrus:algorithm";

const showStudy = (): Result<void, AlgorithmError> => {
    const info: JohnsonInfo = johnsonInfo([[[1, 4], [2, 1]], [[3, 1]], [[1, 2], [3, 5]], []])?;
    print(info.distances[0][3] ?? -1); // 4
    print(info.reweightingCount); // 5
    print(info.bellmanFordRelaxationCount); // 4
    print(info.dijkstraExecutionCount); // 4
    return Ok();
};

match (showStudy()) {
    Ok(_) => {}
    Err(error) => { printError(error.code); // ERR_ALGORITHM_NEGATIVE_CYCLE
    }
}
```
