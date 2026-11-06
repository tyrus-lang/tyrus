```tyrus
const main = (): void => {
    const args: string[] = process.argv.slice(1);
    const target: string = args.at(0) ?? "world";
    print(`hello ${target}`);
};
```
