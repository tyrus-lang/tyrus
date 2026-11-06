```tyrus
const net = Graph<i32, f64>();
net.addEdge(1, 2, 1.5);
net.addEdge(2, 3, 2.5);

print(net.edge(1, 2) ?? 0.0);  // 1.5
print(net.degree(2));          // 1
print(net.nodes());            // [1, 2, 3]
```
