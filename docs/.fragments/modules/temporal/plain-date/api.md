```tyrus
import { Duration, PlainDate, plainDateAddDays, plainDateFrom, plainDateSince, plainDateSubtractDuration, plainDateToString, plainDateUntil, plainDateUntilWithOptions, plainDateWith } from "tyrus:temporal";

const date: PlainDate = match plainDateFrom("2026-07-09") { Ok(value) => value, Err(error) => { printError(error.code); return; } };
const text: string = match plainDateToString(date) { Ok(value) => value, Err(error) => { printError(error.code); return; } }; // "2026-07-09"
const next: PlainDate = match plainDateAddDays(date, 1) { Ok(value) => value, Err(error) => { printError(error.code); return; } };
const holiday: PlainDate = match plainDateWith(date, { year: 2026, month: 12, day: 25 }) { Ok(value) => value, Err(error) => { printError(error.code); return; } };
const duration: Duration = { years: 0, months: 0, weeks: 0, days: 1, hours: 0, minutes: 0, seconds: 0, milliseconds: 0, microseconds: 0, nanoseconds: 0 };
const difference: Duration = match plainDateUntil(date, next) { Ok(value) => value, Err(error) => { printError(error.code); return; } };
const elapsed: Duration = match plainDateSince(next, date) { Ok(value) => value, Err(error) => { printError(error.code); return; } };
const previous: PlainDate = match plainDateSubtractDuration(date, duration) { Ok(value) => value, Err(error) => { printError(error.code); return; } };
const rounded: Duration = plainDateUntilWithOptions(date, next, "days", "days", 1, "trunc"); // 1 day
```
