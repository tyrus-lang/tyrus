```tyrus
import { tempDir } from "tyrus:fs";

const dir = tempDir();
print(dir.path); // "/tmp"
```
