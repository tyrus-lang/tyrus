```tyrus
import { DuplexStream, ReadableStream, StreamError, TransformStream, WritableStream } from "tyrus:stream";
```

| Type | Members |
|------|---------|
| `ReadableStream<T>` | `read(): Promise<T?>`, `pipeTo(destination: WritableStream<T>): Promise<void>`, `close(): Promise<void>` |
| `WritableStream<T>` | `write(chunk: T): Promise<void>`, `close(): Promise<void>` |
| `DuplexStream<TRead, TWrite>` | `readonly readable: ReadableStream<TRead>`, `readonly writable: WritableStream<TWrite>`, `close(): Promise<void>` |
| `TransformStream<TInput, TOutput>` | `readonly writable: WritableStream<TInput>`, `readonly readable: ReadableStream<TOutput>`, `close(): Promise<void>` |
| `StreamError` | `readonly code: string`, `readonly message: string` |
