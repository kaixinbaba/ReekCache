use std::collections::{HashMap, VecDeque};

/// 简单的对标准库中的 VecDeque 进行封装，简化上层调用
struct LRU<T: Ord> {
    ll: VecDeque<T>,
}

impl<T: Ord> LRU<T> {
    fn new() -> Self {
        LRU {
            ll: VecDeque::new(),
        }
    }

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

/// 为了在缓存中存储字节，所以只要能转换成字节数组的类型都是允许的
pub(crate) struct Cache<T: Into<Vec<u8>>> {
    max_size: usize,
    cache: HashMap<String, T>,
    lru: LRU<String>,
    on_evicted: Option<Box<dyn Fn(String, T)>>,
}


impl<T: Into<Vec<u8>>> Cache<T> {
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

    pub(crate) fn get(&mut self, key: &str) -> Option<&T> {
        if let Some(x) = self.cache.get(key) {
            self.lru.move_to_front(key.to_string());
            return Some(x);
        }
        None
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

    fn remove_oldest(&mut self) {
        if let Some(key) = self.lru.remove_oldest() {
            let value = self.cache.remove(&key).unwrap();
            if let Some(on_evicted) = &self.on_evicted {
                on_evicted(key, value);
            }
        }
    }

    pub(crate) fn size(&self) -> usize {
        self.cache.len()
    }
}


#[cfg(test)]
mod tests {
    use std::cell::{RefCell, RefMut};
    use std::rc::Rc;

    use super::*;

    #[test]
    fn test1() {
        let x: Vec<u8> = "123".to_string().into();
        println!("{:?}", x);
    }

    #[test]
    fn test_add() {
        let mut cache = Cache::<&str>::new(2);
        cache.add("123", "11");
        cache.add("23", "12");
        assert_eq!(cache.get("123"), Some(&"11"));
        assert_eq!(cache.get("23"), Some(&"12"));
        cache.add("45", "13");
        assert_eq!(cache.get("45"), Some(&"13"));
        assert_eq!(cache.get("123"), None);
        assert_eq!(cache.get("none"), None);
    }

    #[test]
    fn test_with_evicted() {
        let mut cache = Cache::<&str>::new_with_evicted(2, Box::new(|key, value| {
            println!("removed key: {:?} -> value: {:?}", key, value);
        }));
        cache.add("123", "11");
        cache.add("23", "11");
        cache.add("45", "11");
        cache.get("23");
        cache.add("xjj", "11");
    }
}
