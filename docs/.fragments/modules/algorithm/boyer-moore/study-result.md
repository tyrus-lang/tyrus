```tyrus
interface BoyerMooreInfo {
    text: string;
    pattern: string;
    matches: usize[];
    comparisonCount: usize;
    shiftCount: usize;
    badCharacterTable: [usize, usize][];
    appliedShifts: usize[];
}
```

```tyrus
import { BoyerMooreInfo, boyerMooreInfo } from "tyrus:algorithm";

const info: BoyerMooreInfo = boyerMooreInfo("ababa", "aba");
print(info.text); // ababa
print(info.pattern); // aba
print(info.matches); // [0, 2]
print(info.comparisonCount); // 7
print(info.shiftCount); // 3
print(info.badCharacterTable); // [[97, 2], [98, 1]]
print(info.appliedShifts); // [1, 1, 1]
```
