```tyrus
import { PlainYearMonth, TemporalError, plainYearMonthAdd, plainYearMonthFrom, plainYearMonthToString } from "tyrus:temporal";

export const example = (): void => {
    const value: PlainYearMonth = match plainYearMonthFrom("2026-07") { Ok(result) => result, Err(error) => { printError(error.code); return; } };
    const next: PlainYearMonth = match plainYearMonthAdd(value, 6) { Ok(result) => result, Err(error) => { printError(error.code); return; } };
    const text: string = match plainYearMonthToString(next) { Ok(result) => result, Err(error) => { printError(error.code); return; } }; // "2027-01"
};
```
