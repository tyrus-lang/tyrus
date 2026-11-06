```tyrus
interface Config {
    db: {
        host: string;
        port: i32;
    };
    active: boolean;
}

type ConfigPatch = DeepPartial<Config>;
const patch: ConfigPatch = {
    db: {
        host: "localhost"
    }
};
```
