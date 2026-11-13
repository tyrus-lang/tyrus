```tyrus
interface Config {
    port: i32;
}

const show = (config: readonly Config): void => {
    print(config.port);
};

export function main(): void {
    show({ port: 8080 });
}
```
