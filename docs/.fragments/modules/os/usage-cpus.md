```tyrus
import { CpuInfo, cpus } from "tyrus:os";

const cores: CpuInfo[] = cpus();
print(`${cores.length} cores, ${cores[0].model}`);
```
