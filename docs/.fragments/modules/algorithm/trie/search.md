```tyrus
const t = Trie();
t.insert("apple");
t.insert("app");

print(t.search("apple"));    // true
print(t.search("app"));      // true
print(t.search("apricot"));  // false

```
