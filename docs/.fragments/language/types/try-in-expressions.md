```tyrus
const load = (ok: boolean): Result<i32, string> => {
    if (ok) {
        return Ok(40);
    }
    return Err("boom");
};

const next = (ok: boolean): Result<i32, string> => {
    return Ok(load(ok)? + 1);
};

const pick = (ok: boolean, wanted: boolean): Result<i32, string> => {
    return Ok(wanted ? load(ok)? : 0);
};

const main = (): void => {
    const flag = true;
    print(flag ? -1 : 2); // -1
    match (next(true)) {
        Ok(value) => { print(value); } // 41
        Err(error) => { printError(error); }
    }
    match (pick(false, true)) {
        Ok(value) => { print(value); }
        Err(error) => { printError(error); } // boom
    }
};
```
