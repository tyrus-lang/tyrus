```tyrus
const astronaut: string = "a👨‍🚀é";

for (const cluster of astronaut.graphemes()) {
    print(cluster); // "a", "👨‍🚀", "é"
}

print(astronaut.chars().length);      // 5, scalar values
print(astronaut.graphemes().length);  // 3, clusters
```
