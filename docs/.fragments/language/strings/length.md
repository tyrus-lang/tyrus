```tyrus
const message: string = "tyrus";
const count: usize = message.length; // 5

const accented: string = "héllo";
const characters: usize = accented.length; // 5 characters, not 6 bytes

const launch: string = "a🚀b";
const positions: usize = launch.length; // 3, one position per character
```
