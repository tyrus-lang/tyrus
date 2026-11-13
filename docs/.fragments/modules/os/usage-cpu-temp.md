```tyrus
import { cpuTemp } from "tyrus:os";

const celsius: f32? = cpuTemp();
if (celsius) {
    print(`${celsius}°C`);
}

const fahrenheit: f32? = cpuTemp("fahrenheit");
if (fahrenheit) {
    print(`${fahrenheit}°F`);
}
```
