```tyrus
export function main(): void {
    let ports: readonly i32[] = [3000, 3001];
    print(ports[0]);
    ports.push(3002); // [!code error]
}
```
