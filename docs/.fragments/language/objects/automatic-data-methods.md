### `copy()`

```tyrus
interface User { name: string; }

const user: User = { name: "Ada" };
const copied: User = user.copy();
print(copied); // { name: "Ada" }
```

### `equals()`

```tyrus
interface User { name: string; }

const first: User = { name: "Ada" };
const second: User = { name: "Ada" };
const same: boolean = first.equals(second); // true
```

### `toString()`

```tyrus
interface User { name: string; nickname?: string; }

const user: User = { name: "Ada" };
const text: string = user.toString(); // "{ name: \"Ada\", nickname: None }"
```

### `toJson()`

```tyrus
interface User { name: string; scores: i32[]; }

const user: User = { name: "Ada", scores: [10, 20] };
const json: string = user.toJson(); // "{\"name\": \"Ada\",\"scores\": [10,20]}"
const pretty: string = user.toJson({ pretty: true }); // "{\n    \"name\": \"Ada\",\n    \"scores\": [\n        10,\n        20\n    ]\n}"
```

### `Type.fromJson()`

```tyrus
interface User { name: string; }

const result: Result<User, JsonError> = User.fromJson("{\"name\":\"Ada\"}");
if (result) {
    const user: User = result;
    print(user.name); // Ada
}
```
