```tyrus
import { fetch, HttpClientStreamResponse, HttpError } from "tyrus:http";
import { ReadableStream, StreamError } from "tyrus:stream";

async function openDownload(url: string): Promise<ReadableStream<u8[]>> {
    try {
        const result = await fetch.stream(url);
        const response: HttpClientStreamResponse = match result {
            Ok(value) => value,
            Err(error) => { console.error(error.code); throw error; }
        };
        const body: ReadableStream<u8[]> = await response.stream();
        response.close();
        return body;
    } catch (error: HttpError) {
        console.error(error.code);
        throw error;
    }
}

async function download(url: string): Promise<void> {
    const body: ReadableStream<u8[]> = await openDownload(url);

    try {
        const first: u8[]? = await body.read();
    } catch (error: StreamError) {
        console.error(error.code);
    }
}
```
