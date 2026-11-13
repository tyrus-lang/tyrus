```tyrus
import { BinaryHeap } from "tyrus:algorithm";

const heap: BinaryHeap = BinaryHeap.new<i32>({ capacity: 8 });
heap.push(5);
heap.push(1);
heap.push(3);
print(heap.peek());
print("assert:", 1);
print(heap.pop());
print("assert:", 1);
heap.dispose();
```
