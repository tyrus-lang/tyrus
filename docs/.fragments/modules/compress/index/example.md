```tyrus
import { CompressError, CompressFormat, compress, decompress } from "tyrus:compress";
import { utf8 } from "tyrus:encoding";

const main = async (): Promise<void> => {
    const packed = await compress("Hello, Tyrus!", CompressFormat.Gzip) onfail {
        printError(packed.code);
    };
    const restored = await decompress(packed) onfail {
        printError(restored.code);
    };
    print(utf8.decodeLossy(restored)); // Hello, Tyrus!
};
```
