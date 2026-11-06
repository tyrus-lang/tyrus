```tyrus
import { Duration, PlainTime, plainTimeAddNanoseconds, plainTimeFrom, plainTimeNanosecondsUntil, plainTimeSince, plainTimeSubtractDuration, plainTimeToString, plainTimeUntil, plainTimeUntilWithOptions } from "tyrus:temporal";

const time: PlainTime = match plainTimeFrom("14:30:15.500") { Ok(value) => value, Err(error) => { printError(error.code); return; } };
const text: string = match plainTimeToString(time) { Ok(value) => value, Err(error) => { printError(error.code); return; } }; // "14:30:15.5"
const shiftedTime: PlainTime = match plainTimeAddNanoseconds(time, 750_000_000) { Ok(value) => value, Err(error) => { printError(error.code); return; } };
const nanoseconds: i64 = match plainTimeNanosecondsUntil(time, shiftedTime) { Ok(value) => value, Err(error) => { printError(error.code); return; } }; // 750000000
const duration: Duration = { years: 0, months: 0, weeks: 0, days: 0, hours: 1, minutes: 0, seconds: 0, milliseconds: 0, microseconds: 0, nanoseconds: 0 };
const later: PlainTime = match plainTimeSubtractDuration(shiftedTime, duration) { Ok(value) => value, Err(error) => { printError(error.code); return; } };
const difference: Duration = match plainTimeUntil(time, shiftedTime) { Ok(value) => value, Err(error) => { printError(error.code); return; } };
const elapsed: Duration = match plainTimeSince(shiftedTime, time) { Ok(value) => value, Err(error) => { printError(error.code); return; } };
const rounded: Duration = match plainTimeUntilWithOptions(time, shiftedTime, "hours", "minutes", 15, "halfExpand") { Ok(value) => value, Err(error) => { printError(error.code); return; } }; // 0h 45m
```
