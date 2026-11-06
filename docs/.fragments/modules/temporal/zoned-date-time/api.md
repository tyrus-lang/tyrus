```tyrus
import { Duration, Instant, TemporalError, ZonedDateTime, zonedDateTimeAddDuration, zonedDateTimeEquals, zonedDateTimeFormat, zonedDateTimeFrom, zonedDateTimeToInstant, zonedDateTimeToString, zonedDateTimeUntil } from "tyrus:temporal";

export const example = (): void => {
    const meeting: ZonedDateTime = match zonedDateTimeFrom("2026-07-09T14:30:00-03:00[America/Sao_Paulo]") { Ok(value) => value, Err(error) => { printError(error.code); return; } };
    const instant: Instant = match zonedDateTimeToInstant(meeting) { Ok(value) => value, Err(error) => { printError(error.code); return; } };
    const text: string = match zonedDateTimeToString(meeting) { Ok(value) => value, Err(error) => { printError(error.code); return; } }; // "2026-07-09T14:30:00-03:00[America/Sao_Paulo]"
    const duration: Duration = { years: 0, months: 0, weeks: 0, days: 0, hours: 1, minutes: 0, seconds: 0, milliseconds: 0, microseconds: 0, nanoseconds: 0 };
    const later: ZonedDateTime = match zonedDateTimeAddDuration(meeting, duration) { Ok(value) => value, Err(error) => { printError(error.code); return; } };
    const same: boolean = zonedDateTimeEquals(meeting, meeting); // true
    const difference: Duration = match zonedDateTimeUntil(meeting, later) { Ok(value) => value, Err(error) => { printError(error.code); return; } };
    const formatted: string = match zonedDateTimeFormat(later, "yyyy-MM-dd HH:mm") { Ok(value) => value, Err(error) => { printError(error.code); return; } }; // "2026-07-09 15:30"
};
```
