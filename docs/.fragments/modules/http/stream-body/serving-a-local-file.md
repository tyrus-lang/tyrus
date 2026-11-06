```tyrus
const source: ReadableStream<u8[]> = readFileStream("media/recording-source.bin");
reply.type("video/mp4").status(HttpStatus.Ok).send(source);
```
