```tyrus
import { Duration, PlainDate, TemporalError, durationFromFields, durationRound, durationTotal, plainDateAddDuration, plainDateFrom } from "tyrus:temporal";

export const example = (): void => {
    const duration: Duration = durationFromFields({ years: 0, months: 0, weeks: 0, days: 0, hours: 1, minutes: 30, seconds: 0, milliseconds: 0, microseconds: 0, nanoseconds: 0 });
    const minutes: i64 = match durationTotal(duration, "minutes") { Ok(value) => value, Err(error) => { printError(error.code); return; } }; // 90
    const date: PlainDate = match plainDateFrom("2026-07-09") { Ok(value) => value, Err(error) => { printError(error.code); return; } };
    const oneDay: Duration = durationFromFields({ years: 0, months: 0, weeks: 0, days: 1, hours: 0, minutes: 0, seconds: 0, milliseconds: 0, microseconds: 0, nanoseconds: 0 });
    const result: PlainDate = match plainDateAddDuration(date, oneDay) { Ok(value) => value, Err(error) => { printError(error.code); return; } }; // 2026-07-10
    const rounded: Duration = match durationRound(duration, "minutes", 15, "halfExpand", date) { Ok(value) => value, Err(error) => { printError(error.code); return; } }; // 1h 30m
};
```
