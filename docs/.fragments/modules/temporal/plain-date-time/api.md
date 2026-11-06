```tyrus
import { Duration, PlainDateTime, TemporalError, plainDateTimeAddDuration, plainDateTimeFrom, plainDateTimeToString, plainDateTimeUntil, plainDateTimeUntilWithOptions } from "tyrus:temporal";

export const example = (): void => {
    const value: PlainDateTime = match plainDateTimeFrom("2026-07-09T23:30:00") { Ok(result) => result, Err(error) => { printError(error.code); return; } };
    const text: string = match plainDateTimeToString(value) { Ok(result) => result, Err(error) => { printError(error.code); return; } }; // "2026-07-09T23:30:00"
    const duration: Duration = { years: 0, months: 0, weeks: 0, days: 0, hours: 1, minutes: 0, seconds: 0, milliseconds: 0, microseconds: 0, nanoseconds: 0 };
    const next: PlainDateTime = match plainDateTimeAddDuration(value, duration) { Ok(result) => result, Err(error) => { printError(error.code); return; } }; // 2026-07-10T00:30:00
    const difference: Duration = match plainDateTimeUntil(value, next) { Ok(result) => result, Err(error) => { printError(error.code); return; } };
    const rounded: Duration = match plainDateTimeUntilWithOptions(value, next, "hours", "minutes", 30, "halfExpand") { Ok(result) => result, Err(error) => { printError(error.code); return; } }; // 1h 00m
};
```
