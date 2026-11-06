```tyrus
import { input } from "tyrus:cli";

const name: string = input({
    message: "Project name?",
    pattern: /^[a-z][a-z0-9-]*$/,
    patternMessage: "lowercase, start with a letter",
});
```
