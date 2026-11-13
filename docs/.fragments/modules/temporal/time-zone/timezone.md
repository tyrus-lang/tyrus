```tyrus
import { TemporalError, TimeZone, timeZone } from "tyrus:temporal";

export const example = (): void => {
    const paris: TimeZone = match timeZone("Europe/Paris") { Ok(value) => value, Err(error) => { printError(error.code); return; } };
    const saoPaulo: TimeZone = match timeZone("America/Sao_Paulo") { Ok(value) => value, Err(error) => { printError(error.code); return; } };
};
```
