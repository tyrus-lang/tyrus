```tyrus
import { FsError, readFileSync } from "tyrus:fs";

interface ConfigError extends Error {
    path: string;
}

function readText(path: string): string throws FsError {
    let contents: string = "";
    match (readFileSync(path)) {
        Ok(text) => { contents = text; }
        Err(error) => { throw error; }
    }
    return contents;
}

function loadConfig(path: string): string throws ConfigError {
    const failure: ConfigError = { message: "config is missing", code: "ERR_CONFIG", path: path };
    throw failure;
}

function readSettings(): string throws Error {
    try {
        return readText("settings.json") + loadConfig("app.json");
    } catch (error) {
        if (error is ConfigError) {
            console.error(`${error.code}: ${error.path}`);
            return "";
        }
        // What this handler does not recognize goes on, subtype and all.
        throw error;
    }
}

const main = (): void => {
    try {
        console.log(readSettings());
    } catch (error) {
        if (error is FsError) {
            console.error(`${error.code}: ${error.message}`);
        } else {
            console.error(error.message);
        }
    }
};
```
