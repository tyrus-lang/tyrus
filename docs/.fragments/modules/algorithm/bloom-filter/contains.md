```tyrus
const bf = BloomFilter(3, 256);
bf.insert("apple");
bf.insert("banana");
bf.insert("cherry");
bf.insert("date");

print(bf.contains("apple"));   // true
print(bf.contains("banana"));  // true
print(bf.contains("egg"));     // false
print(bf.contains("fig"));     // false
print(bf.contains("grape"));   // false
print(bf.contains("honey"));   // false

```
