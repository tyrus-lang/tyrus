```tyrus
import { platform } from "tyrus:os";
import { OsPlatform } from "tyrus:os";

const currentPlatform: OsPlatform = platform();

if (currentPlatform === OsPlatform.Windows) {
    print("running on Windows");
}
```
