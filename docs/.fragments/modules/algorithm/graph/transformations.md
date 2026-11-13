```tyrus
const doubled = g.mapEdges((w: i32): i32 => w * 2);
print(doubled.edge("b", "c") ?? 0);  // 4

const upper = g.mapNodes((n: string): string => n.toUpperCase());
print(upper.hasNode("A"));  // true
print(upper.edge("A", "C") ?? 0);  // 6

const reversed = g.reverse();
print(reversed.hasEdge("c", "b"));  // true
print(reversed.hasEdge("b", "c"));  // false
```
