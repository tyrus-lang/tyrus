```tyrus
import { mime } from "tyrus:fs";

print(mime("file.txt")); // "text/plain"
print(mime("image.png")); // "image/png"
print(mime("file.xyz")); // null
```
