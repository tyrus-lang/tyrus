```tyrus
import { connectWebSocket, createRouter, HttpError, HttpRouter, HttpServer, WebSocket, WebSocketMessage } from "tyrus:http";

async function echoMessages(socket: WebSocket): Promise<void> throws HttpError {
    let open: boolean = true;
    while (open) {
        const message: WebSocketMessage? = await socket.receive();
        if (message) {
            if (message.binary) {
                await socket.sendBytes(message.bytes);
            } else {
                await socket.sendText(`echo ${message.text}`);
            }
        } else {
            open = false;
        }
    }
}

async function echo(socket: WebSocket): Promise<void> {
    try {
        await echoMessages(socket);
    } catch (error: HttpError) {
        console.error(error.code);
    }
}

async function main(): Promise<void> {
    const app: HttpRouter = createRouter();
    app.ws("/echo", { handler: echo, origins: ["https://app.example.com"], subprotocols: ["chat"], maxMessageBytes: 65536 });
    try {
        const server: HttpServer = app.start(3000);
        const connection = await connectWebSocket("ws://127.0.0.1:3000/echo", ["chat"]);
        const socket: WebSocket = match connection {
            Ok(value) => value,
            Err(error) => { console.error(error.code); return; }
        };
        await socket.sendText("hello");
        const reply: WebSocketMessage? = await socket.receive();
        if (reply) {
            console.log(reply.text);
        }
        await socket.close();
        await server.close();
    } catch (error: HttpError) {
        console.error(error.code);
    }
}
```
