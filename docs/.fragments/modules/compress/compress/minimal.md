```tyrus
import { CompressError, CompressFormat, compress } from "tyrus:compress";

async function shrink(): Promise<Result<void, CompressError>> {
    const packed: u8[] = await compress("Hello, Tyrus!", CompressFormat.Gzip)?;
    print(packed[0], packed[1]); // 31 139
    return Ok();
}
```
