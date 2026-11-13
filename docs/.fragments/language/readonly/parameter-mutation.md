```tyrus
const bump = (config: readonly Config): void => {
    config.port = 9090; // [!code error]
};
```
