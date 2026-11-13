```tyrus
print(g.toString());
// Graph { "a" -> "b" [label="1"], "a" -> "c" [label="3"], "b" -> "c" [label="2"] }

print(g.toDot());
// digraph {
//     "a" -> "b" [label="1"];
//     "a" -> "c" [label="3"];
//     "b" -> "c" [label="2"];
// }
```
