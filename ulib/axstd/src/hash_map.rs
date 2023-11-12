use hashbrown::hash_map as base;

#[allow(deprecated)]
use core::hash::{BuildHasher, Hash, Hasher, SipHasher13};

/// HashMap with default Hasher
pub struct HashMap<K, V, S = RandomState> {
    base: base::HashMap<K, V, S>,
}

impl<K, V> HashMap<K, V, RandomState> {
    /// Creates an empty HashMap which will use the default hash builder to hash keys.
    #[inline]
    #[must_use]
    pub fn new() -> HashMap<K, V, RandomState> {
        HashMap::with_hasher(RandomState::new())
    }
}

impl<K, V, S> HashMap<K, V, S> {
    /// Creates an empty HashMap which will use the given hash builder to hash keys.
    #[inline]
    pub const fn with_hasher(hash_builder: S) -> HashMap<K, V, S> {
        HashMap {
            base: base::HashMap::with_hasher(hash_builder),
        }
    }

    /// An iterator visiting all key-value pairs in arbitrary order. The iterator element type is (&'a K, &'a V).
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            base: self.base.iter(),
        }
    }
}

impl<K, V, S> HashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    /// Inserts a key-value pair into the map.  
    /// If the map did not have this key present, `None` is returned.  
    /// If the map did have this key present, the value is updated, and the old value is returned.
    #[inline]
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.base.insert(k, v)
    }
}

/// An iterator over the entries of a `HashMap`, created by the [`iter`] method on [`HashMap`].
pub struct Iter<'a, K: 'a, V: 'a> {
    base: base::Iter<'a, K, V>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    #[inline]
    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        self.base.next()
    }
}

/// default state for [`HashMap`] types.
#[derive(Clone)]
pub struct RandomState {
    k0: u64,
    k1: u64,
}

impl RandomState {
    /// Constructs a new `RandomState` that is initialized with random keys.  
    /// simply call `rand::thread_rng().gen()` twice.
    #[inline]
    #[must_use]
    pub fn new() -> RandomState {
        use arceos_api::random;
        let randu128 = random(); 
        RandomState { k0: randu128 as u64, k1: (randu128 >> 64) as u64 }
    }
}

impl BuildHasher for RandomState {
    type Hasher = DefaultHasher;
    #[inline]
    #[allow(deprecated)]
    fn build_hasher(&self) -> DefaultHasher {
        DefaultHasher(SipHasher13::new_with_keys(self.k0, self.k1))
    }
}

/// default [`Hasher`] used by [`RandomState`].
#[allow(deprecated)]
pub struct DefaultHasher(SipHasher13);

impl Hasher for DefaultHasher {
    /// Writes some data into this `Hasher`.
    #[inline]
    fn write(&mut self, msg: &[u8]) {
        self.0.write(msg)
    }

    /// Returns the hash value for the values written so far.
    #[inline]
    fn finish(&self) -> u64 {
        self.0.finish()
    }
}

