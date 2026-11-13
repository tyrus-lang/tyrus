```tyrus
import { multiselect } from "tyrus:cli";

const features: string[] = multiselect({
    message: "Features?",
    choices: ["http", "fmt", "lint"],
});
```
