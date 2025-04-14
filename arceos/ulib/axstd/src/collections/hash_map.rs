use alloc::vec::Vec;
use core::hash::{BuildHasher, Hash, Hasher};
const FNV: u64 = 0x100000001b3;

use axhal::misc::random;
struct FnvHasher(u64);
impl Hasher for FnvHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(FNV);
        }
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

struct FnvBuildHasher;
impl BuildHasher for FnvBuildHasher {
    type Hasher = FnvHasher;

    fn build_hasher(&self) -> Self::Hasher {
        FnvHasher(random() as u64)
    }
}

const BUCKET_INIT_SIZE: usize = 8;
const EXPAND_RATE: f64 = 0.5;
pub struct HashMap<K: Hash, V> {
    data: Vec<Option<(K, V)>>,
    capacity: usize,
    size: usize,
}

impl<K: Hash, V> HashMap<K, V> {
    pub fn new() -> Self {
        let mut data = Vec::with_capacity(BUCKET_INIT_SIZE);
        data.resize_with(BUCKET_INIT_SIZE, || None);
        println!("{}", data.len());
        Self {
            data,
            capacity: BUCKET_INIT_SIZE,
            size: 0,
        }
    }
    pub fn should_expand(&self) -> bool {
        (self.size as f64 / self.capacity as f64) >= EXPAND_RATE
    }
    pub fn hash_key(&self, key: &K) -> u64 {
        let mut hasher = FnvBuildHasher.build_hasher();
        key.hash(&mut hasher);
        hasher.finish()
    }
    pub fn insert(&mut self, key: K, val: V) {
        if self.should_expand() {
            // expand
            println!("expand from {} to {}", self.capacity, self.capacity * 2);
            self.capacity *= 2;

            let mut trans_data = Vec::with_capacity(self.capacity);
            trans_data.resize_with(self.capacity, || None);
            core::mem::swap(&mut trans_data, &mut self.data);

            assert!(!self.data.iter().all(|x| x.is_some()));

            /*
            trans_data
                .into_iter()
                .filter_map(|e| e)
                .for_each(|(k, v)| self.insert(k, v));
            */
            self.size = 0;
            for (k, v) in trans_data.into_iter().flatten() {
                let mut pos = self.hash_key(&k) as usize % self.capacity;
                loop {
                    if self.data[pos].is_none() {
                        self.data[pos] = Some((k, v));
                        self.size += 1;
                        break;
                    }
                    pos = (pos + 1) % self.capacity;
                }
            }
            println!("expanded ok.");
        }
        let mut expected_pos = self.hash_key(&key) as usize % self.capacity;
        // println!("got hash value: {}", expected_pos);
        let original = expected_pos;
        loop {
            if self.data.iter().all(|x| x.is_some()) {
                panic!("...");
            }
            /*
            println!(
                "now expected_pos: {}, size = {}",
                expected_pos,
                self.data.len()
            );
            */
            if self.data.get_mut(expected_pos).unwrap().is_none() {
                self.size += 1;
                self.data[expected_pos] = Some((key, val));
                return;
            }
            expected_pos = (expected_pos + 1) as usize % self.capacity;
        }
    }
    pub fn iter(&self) -> Iter<K, V> {
        Iter {
            data: &self.data,
            pos: 0,
        }
    }
}

pub struct Iter<'iter, K, V> {
    data: &'iter Vec<Option<(K, V)>>,
    pos: usize,
}

impl<'iter, K, V> Iterator for Iter<'iter, K, V> {
    type Item = &'iter (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.data.len() {
            if let Some(ref e) = self.data[self.pos] {
                self.pos += 1;
                return Some(e);
            }
            self.pos += 1;
        }
        None
    }
}
