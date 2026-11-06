```tyrus
import { cpuTemp } from "tyrus:os";

// Stands in for a config file, an HTTP response, a CLI argument, any
// value only known once the program is running.
const readPreferredUnit = (): string => {
  return "fahrenheit";
};

const main = (): void => {
  const preferredUnit: string = readPreferredUnit();

  // `preferredUnit` is a plain `string`, not the literal union `cpuTemp`
  // expects, so it cannot be passed through directly, narrow it to one
  // of the two accepted literals first.
  let temp: f32?;
  if (preferredUnit === "fahrenheit") {
    temp = cpuTemp("fahrenheit");
  } else {
    temp = cpuTemp("celsius");
  }
  if (temp) {
    print(temp);
  }
};
```
