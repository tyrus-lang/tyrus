```tyrus
interface Cursor {
    row: i32;
    column: i32;
    visible: boolean;
}

async function move(): Promise<void> {
    let cursor: Cursor = { row: 1, column: 2, visible: true };
    await Promise.resolve();
    cursor.row = 3;
    cursor = { row: 4, column: 5, visible: false };
    await Promise.resolve();
    print(cursor.column);
}
```
