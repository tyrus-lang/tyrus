```tyrus
import { integer } from "tyrus:cli";

const workers: i64 = integer({
    message: "Worker count?",
    default: 4,
    min: 1,
    max: 64,
});
```
