# 第一天 LRU 缓存淘汰策略
又开了新坑，之前在网上看到了 [极客兔兔](https://github.com/geektutu) 的 7 天教程，看了以后觉得很不错，决定照猫画虎弄一个 Rust 版本的。
> 事先声明：我只是 Rust 的初学者，代码肯定有非常多优化的空间，希望大佬指出，另外由于是致敬之作，所以整体文章的流程还是遵照了原版 GoLang 的路线。
> 本系列不会赘述 Rust 的基础语法等，需要读者具有一定的 Rust 基础，以及一些 geektutu 已经介绍过的概念，项目整体还是一切从简
> 最后由于是致敬之作，项目名称我都会以 Reek 开头，取自 Rust + geektutu。

## 一、创建项目
```
$ cargo new ReekCache
```
就会得到一个如下目录结构
```
$ tree ReekCache
ReekCache
├── Cargo.toml
├── src
  └── main.rs
```
然后我们尝试修改下 `main.rs` 
```rust
fn main() {
    println!("Hello ReekCache!")
}
```
然后运行 `cargo run`，不出意外就能得到以下输出
```
...
Hello ReekCache!
```

## 二、创建 lru 模块
[lru.rs](../src/lru.rs)
和原版教程一样，先创建一个缓存真正的底层实现，用来存储数据并且维护一个最近最少使用（lru）的队列，用于在缓存满了的时候，
并且拥有一个可选的回调函数用来处理被删除的元素

### 2.1 声明结构体
```rust
use std::collections::{HashMap, VecDeque};

/// 简单的对标准库中的 VecDeque 进行封装，简化上层调用
struct LRU<T: Ord> {
    ll: VecDeque<T>,
}

pub(crate) struct Cache<T> {
    max_size: usize,
    cache: HashMap<String, T>,
    lru: LRU<String>,
    on_evicted: Option<Box<dyn Fn(String, T)>>,
}
```
- `Cache` 没有直接使用 `VecDeque`，而是使用自己封装的 `LRU` 结构体
- `LRU` 的泛型其实可以不需要，因为存放的肯定是缓存的 key 类型 `String`
- `max_size` 没有按照原版的教程中记录的是字节，而是缓存的 key 数量，淘汰策略也是根据当前 key 数量来判断的，我个人觉得更简单，一切从简出发

### 2.2 构造函数
为两个自定义结构体都声明构造函数
```rust
impl<T: Ord> LRU<T> {
    fn new() -> Self {
        LRU {
            ll: VecDeque::new(),
        }
    }
}

impl<T> Cache<T> {
    pub(crate) fn new(max_size: usize) -> Self {
        Cache {
            max_size,
            cache: HashMap::new(),
            lru: LRU::new(),
            on_evicted: None,
        }
    }

    pub(crate) fn new_with_evicted(max_size: usize, on_evicted: Box<dyn Fn(String, T)>) -> Self {
        Cache {
            max_size,
            cache: HashMap::new(),
            lru: LRU::new(),
            on_evicted: Some(on_evicted),
        }
    }
}
```
- Rust 是没有 GoLang 中类似 `nil` 的空值的，可能为空的字段或者变量需要显式的用 `Option` 包装以下
- Rust 的构造器就是普通的函数，一般使用 new 命名，如果有可选参数一般会加 with_xxx

### 2.3 LRU 的其他方法
```rust
impl<T: Ord> LRU<T> {
    // ...

    fn push(&mut self, value: T) {
        self.ll.push_front(value);
    }

    fn move_to_front(&mut self, value: T) {
        if let Ok(index) = self.ll.binary_search(&value) {
            let value = self.ll.remove(index).unwrap();
            self.ll.push_front(value);
        }
    }
    pub(crate) fn remove_oldest(&mut self) -> Option<T> {
        self.ll.pop_back()
    }
}
```
因为原版教程中 GoLang 标准库的链表是拥有一些函数的，这里我们只能自己封装下 LRU

### 2.4 查找功能
查找主要有 2 个步骤，第一步是从字典中找到对应的双向链表的节点，第二步，将该节点移动到队尾。
```rust
impl<T> Cache<T> {

    pub(crate) fn get(&mut self, key: &str) -> Option<&T> {
        if let Some(x) = self.cache.get(key) {
            self.lru.move_to_front(key.to_string());
            return Some(x);
        }
        None
    }
}
```
- 因为 Rust 涉及到所有权，所以 `get` 方法返回的是 引用`&`
- 如果 key 不存在的话直接返回 `None`，存在的话就需要把当前 key 移动至 LRU 的队首

### 2.5 删除功能
```rust
impl<T> Cache<T> {

    fn remove_oldest(&mut self) {
        if let Some(key) = self.lru.remove_oldest() {
            let value = self.cache.remove(&key).unwrap();
            if let Some(on_evicted) = &self.on_evicted {
                on_evicted(key, value);
            }
        }
    }
}
```
- 删除队尾元素，并且同时删除哈希表中的键值对
- 如果有回掉函数则调用它，因为 Rust 中赋值操作 `=` 会转移所有权，所以这里只能用引用 `&self.on_evicted`

### 2.6 新增/修改
```rust
impl<T> Cache<T> {

    pub(crate) fn size(&self) -> usize {
        self.cache.len()
    }

    pub(crate) fn add(&mut self, key: &str, value: T) {
        match self.cache.insert(key.to_string(), value) {
            None => {
                // 新增
                self.lru.push(key.to_string());
            }
            Some(_) => {
                // 修改
                self.lru.move_to_front(key.to_string());
            }
        }
        // 超过上限就要删除元素并回调
        if self.size() > self.max_size {
            self.remove_oldest();
        }
    }
}
```
- 整个 Cache 或者是 LRU 的操作都没有考虑到并发，都是直接操作的，因为这两个结构体的定位都是内部使用的所以并发安全部分会由上层去处理

## 三、测试
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        let mut cache = Cache::<i32>::new(2);
        cache.add("123", 11);
        cache.add("23", 12);
        assert_eq!(cache.get("123"), Some(&11));
        assert_eq!(cache.get("23"), Some(&12));
        cache.add("45", 13);
        assert_eq!(cache.get("45"), Some(&13));
        assert_eq!(cache.get("123"), None);
        assert_eq!(cache.get("none"), None);
    }

    #[test]
    fn test_with_evicted() {
        let mut cache = Cache::<i32>::new_with_evicted(2, Box::new(|key, value| {
            println!("removed key: {:?} -> value: {:?}", key, value);
        }));
        cache.add("123", 11);
        cache.add("23", 11);
        cache.add("45", 11);
        cache.get("23");
        cache.add("xjj", 11);
    }
}
```
执行下 `cargo test` 就可以看到
```
...
running 2 tests
test lru::tests::test_add ... ok
test lru::tests::test_with_evicted ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```