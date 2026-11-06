```tyrus
interface User {
    name: string;
}

const loadUser = async (id: i32): Promise<Result<User, string>> => {
    if (id === 1) {
        const user: User = { name: "Ada" };
        return Ok(user);
    }
    return Err(`no user ${id}`);
};

const greeting = async (id: i32): Promise<Result<string, string>> => {
    const user = await loadUser(id)?;
    return Ok(`hello ${user.name}`);
};

const main = async (): Promise<void> => {
    for (let id = 1; id <= 2; id += 1) {
        match (await greeting(id)) {
            Ok(text) => { print(text); } // hello Ada
            Err(error) => { printError(error); } // no user 2
        }
    }
};
```
