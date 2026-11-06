```tyrus
import { PlainDate, PlainMonthDay, TemporalError, plainMonthDayFrom, plainMonthDayToPlainDate, plainMonthDayToString } from "tyrus:temporal";

export const example = (): void => {
    const birthday: PlainMonthDay = match plainMonthDayFrom("02-29") { Ok(result) => result, Err(error) => { printError(error.code); return; } };
    const text: string = match plainMonthDayToString(birthday) { Ok(result) => result, Err(error) => { printError(error.code); return; } }; // "02-29"
    const date: PlainDate = match plainMonthDayToPlainDate(birthday, 2028) { Ok(result) => result, Err(error) => { printError(error.code); return; } }; // 2028-02-29
};
```
