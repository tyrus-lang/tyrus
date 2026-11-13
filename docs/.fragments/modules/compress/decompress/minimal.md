```tyrus
import { CompressError, CompressFormat, compress, decompress } from "tyrus:compress";

async function restore(): Promise<Result<void, CompressError>> {
    const packed: u8[] = await compress("Hello, Tyrus!", CompressFormat.Gzip)?;
    const original: u8[] = await decompress(packed)?;
    print(original.length); // 13
    return Ok();
}
```
