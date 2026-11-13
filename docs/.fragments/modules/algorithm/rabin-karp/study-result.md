```tyrus
interface RabinKarpInfo {
    text: string;
    pattern: string;
    matches: usize[];
    comparisonCount: usize;
    shiftCount: usize;
    windowCount: usize;
    hashCollisionCount: usize;
    textVerificationCount: usize;
}
```

```tyrus
import { RabinKarpInfo, StringSearchInfo, rabinKarpInfo } from "tyrus:algorithm";
const info: RabinKarpInfo = rabinKarpInfo("ababa", "ba");
print(info.matches); // [1, 3]
print(info.windowCount); // 4
print(info.hashCollisionCount); // 0
print(info.textVerificationCount); // 2
```
