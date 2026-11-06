```tyrus
interface Message {
    text?: string;
}

const message: Message = {};
const text: string = message.text ?? "default";
```
