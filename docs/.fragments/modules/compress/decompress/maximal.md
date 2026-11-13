```tyrus
import { CompressError, CompressFormat, DecompressOptions, decompress } from "tyrus:compress";

async function restore(packed: u8[]): Promise<Result<void, CompressError>> {
    const original: u8[] = await decompress(packed, {
        format: CompressFormat.Brotli, // required for Brotli and DeflateRaw; default detected from the data
        maxOutputBytes: 64_000_000, // refuse more output than this; default 256 MiB
        maxRatio: 1_000, // refuse more than this expansion once past 1 MiB; default 200
    })?;
    print(original.length); // 13
    return Ok();
}
```
