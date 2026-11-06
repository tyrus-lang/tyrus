```tyrus
interface User { name: string; }

function main(): void {
    const json = User.fromJson("{\"name\":123}") onfail {
        print(json.code); // ERR_JSON_TYPE_MISMATCH
        print(json.message); // JSON field does not match the declared type
        print(json.path); // name
    };

    print(json.name); //
}
```
