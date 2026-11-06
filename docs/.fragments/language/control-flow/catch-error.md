```tyrus
function load(): void throws Error {
    const failure: Error = { message: "config file is missing", code: "ERR_CONFIG_MISSING" };
    throw failure;
}

const main = (): void => {
    try {
        load();
    } catch (error) {
        console.error(`${error.code}: ${error.message}`);
    }
};
```
