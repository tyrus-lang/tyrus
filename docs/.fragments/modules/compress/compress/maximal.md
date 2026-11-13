```tyrus
import { CompressError, CompressFormat, CompressOptions, compress } from "tyrus:compress";

async function shrink(report: string): Promise<Result<void, CompressError>> {
    const packed: u8[] = await compress(report, CompressFormat.Brotli, {
        level: 11, // 0 to 11 for Brotli, 0 to 9 for the others; default 6
    })?;
    print(packed.length < report.length); // true
    return Ok();
}
```
