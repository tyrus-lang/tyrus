```tyrus
import { fetch, HttpClientResponse, HttpError } from "tyrus:http";
import { FsError, readFileStream } from "tyrus:fs";
import { ReadableStream } from "tyrus:stream";

async function upload(url: string): Promise<void> {
    try {
        const source: ReadableStream<u8[]> = readFileStream("media/recording-source.bin");
        const result = await fetch.post(url, { bodyStream: source });
        const response: HttpClientResponse = match result {
            Ok(value) => value,
            Err(error) => { console.error(error.code); return; }
        };
        response.close();
    } catch (error: FsError) {
        console.error(error.code);
    } catch (error: HttpError) {
        console.error(error.code);
    }
}
```
