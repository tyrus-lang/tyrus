```tyrus
print(g.hasNode("a"));     // true
print(g.hasEdge("a", "b"));  // true
print(g.edge("a", "b") ?? 0);  // 1
print(g.edge("b", "a") ?? 0);  // 0

print(g.degree("a"));  // 2
print(g.neighbors("a"));  // ["b", "c"]
```
