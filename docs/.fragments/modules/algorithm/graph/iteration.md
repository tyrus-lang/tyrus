```tyrus
for (const node of g.nodes()) {
    print(node);
}

for (const [source, target, weight] of g.edges()) {
    print(source, "->", target, "(", weight, ")");
}
```
