```tyrus
interface EditDistanceInfo {
    left: string;
    right: string;
    distance: usize;
    rowCount: usize;
    columnCount: usize;
    evaluatedCellCount: usize;
}
```

```tyrus
import { EditDistanceInfo, levenshteinInfo } from "tyrus:algorithm";

const info: EditDistanceInfo = levenshteinInfo("kitten", "sitting");
print(info.left); // kitten
print(info.right); // sitting
print(info.distance); // 3
print(info.rowCount); // 7
print(info.columnCount); // 8
print(info.evaluatedCellCount); // 42
```
