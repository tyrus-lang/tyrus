```tyrus
import { documentsDir } from "tyrus:os";

const documents: string? = documentsDir();
if (documents) {
    print(`documents: ${documents}`);
}
```
