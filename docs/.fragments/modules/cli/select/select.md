```tyrus
import { select } from "tyrus:cli";

const target: string = select({
    message: "Target?",
    choices: ["script", "cli", "api"],
});
```
