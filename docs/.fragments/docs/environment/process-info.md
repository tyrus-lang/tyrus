```tyrus
const main = (): void => {
    print(`pid ${process.pid}, ppid ${process.ppid}`);
    print(`running from ${process.cwd()}`);
    print(`up for ${process.uptime()}ms`);

    const memory = process.memoryUsage();
    print(`rss ${memory.rss} bytes`);

    const cpu = process.cpuUsage();
    print(`cpu ${cpu.user}us user, ${cpu.system}us system`);
};
```
