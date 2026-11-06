```tyrus
import { confirm } from "tyrus:cli";

const ok: boolean = confirm({
    message: "Continue?",
    default: true,
    yes: "Yes",
    no: "No",
});
```
