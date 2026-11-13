```tyrus
import { EncodingError, hex } from "tyrus:encoding";

const bytes: u8[] = [0xde, 0xad, 0xbe, 0xef];
const encoded: string = hex.encode(bytes);
print(encoded); // deadbeef

try {
    const decoded: u8[] = hex.decode(encoded);
    print(decoded); // [222, 173, 190, 239]
} catch (error: EncodingError) {
    print(error.code);
}
```
