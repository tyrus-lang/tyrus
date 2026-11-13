```tyrus
interface AhoCorasickInfo {
    text: string;
    patterns: string[];
    matches: [usize, usize][];
    visitedStates: usize[];
    transitionCount: usize;
    failureFollowCount: usize;
}
```

```tyrus
import { AhoCorasickInfo, ahoCorasickInfo } from "tyrus:algorithm";

const info: AhoCorasickInfo = ahoCorasickInfo("ababa", ["aba", "ba"]);
print(info.text); // ababa
print(info.patterns); // [aba, ba]
print(info.matches); // [[2, 0], [2, 1], [4, 0], [4, 1]]
print(info.visitedStates); // [1, 2, 3, 2, 3]
print(info.transitionCount); // 5
print(info.failureFollowCount); // 2
```
