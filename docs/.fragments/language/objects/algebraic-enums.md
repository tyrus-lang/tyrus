```tyrus
enum Command {
    Quit,
    Move(i32, i32),
    ChangeColor { r: u8, g: u8, b: u8 },
}

const main = (): void => {
    const describe = (command: Command): i32 => {
        match (command) {
            Command.Quit => { return 0; }
            Command.Move(x, y) => { return x + y; }
            Command.ChangeColor { r, g, b } => { return (r + g + b) as i32; }
        }
    };

    const result = describe(Command.ChangeColor({ r: 10u8, g: 20u8, b: 30u8 }));

    print(result); // 60
};
```
