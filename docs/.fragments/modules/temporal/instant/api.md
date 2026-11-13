```tyrus
import { Duration, Instant, TemporalError, TimeZone, ZonedDateTime, instantAddDuration, instantFrom, instantFromEpochMilliseconds, instantNow, instantToEpochMilliseconds, instantToString, instantToZonedDateTime, timeZone } from "tyrus:temporal";

export const example = (): void => {
    const now: Instant = instantNow();
    const parsed: Instant = match instantFrom("2026-07-09T15:30:00Z") { Ok(value) => value, Err(error) => { printError(error.code); return; } };
    const epoch: Instant = match instantFromEpochMilliseconds(0) { Ok(value) => value, Err(error) => { printError(error.code); return; } };
    const milliseconds: i64 = match instantToEpochMilliseconds(epoch) { Ok(value) => value, Err(error) => { printError(error.code); return; } }; // 0
    const text: string = match instantToString(epoch) { Ok(value) => value, Err(error) => { printError(error.code); return; } }; // "1970-01-01T00:00:00Z"
    const second: Duration = { years: 0, months: 0, weeks: 0, days: 0, hours: 0, minutes: 0, seconds: 1, milliseconds: 0, microseconds: 0, nanoseconds: 0 };
    const later: Instant = match instantAddDuration(epoch, second) { Ok(value) => value, Err(error) => { printError(error.code); return; } };
    const paris: TimeZone = match timeZone("Europe/Paris") { Ok(value) => value, Err(error) => { printError(error.code); return; } };
    const zoned: ZonedDateTime = instantToZonedDateTime(parsed, paris);
};
```
