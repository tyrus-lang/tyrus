```tyrus
interface SpfaInfo {
    distances: i32[];
    predecessors: usize?[];
    relaxationCount: usize;
    processedVertexCount: usize;
    examinedEdgeCount: usize;
    queuePushCount: usize;
    queuePopCount: usize;
    maxQueueSize: usize;
}
```

```tyrus
import { AlgorithmError, SpfaInfo, spfaInfo } from "tyrus:algorithm";

const showStudy = (): Result<void, AlgorithmError> => {
    const info: SpfaInfo = spfaInfo([[[1, 4], [2, 1]], [[3, 1]], [[1, 2], [3, 5]], []], 0)?;
    print(info.distances); // [0, 3, 1, 4]
    print(info.predecessors[3] ?? 99); // 1
    print(info.relaxationCount); // 5
    print(info.processedVertexCount); // 4
    print(info.examinedEdgeCount); // 6
    print(info.queuePushCount); // 6
    print(info.queuePopCount); // 6
    print(info.maxQueueSize); // 2
    return Ok();
};

match (showStudy()) {
    Ok(_) => {}
    Err(error) => { printError(error.code); // ERR_ALGORITHM_NEGATIVE_CYCLE
    }
}
```
