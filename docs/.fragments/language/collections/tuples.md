```tyrus
const point: [f64, f64] = [3.0, 4.0];
const [x, y] = point;
const pair: [i32, string] = [1, "one"];
const namedPair: Tuple<i32, string> = [1, "one"];

const parseStatus = (): [i32, string] => [200, "ok"];
const [statusCode, _] = parseStatus();
print(statusCode); // 200
```
