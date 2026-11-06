```tyrus
import { password } from "tyrus:cli";

const secret: string = password({
    message: "Type a password",
    confirm: true,
    confirmMessage: "Confirm password",
});
```
