#[cfg(test)]
mod tests;

use self::Entry::*;

use hashbrown::hash_map as base;

use crate::borrow::Borrow;
use crate::cell::Cell;
use crate::collections::TryReserveError;
use crate::collections::TryReserveErrorKind;
use crate::error::Error;
use crate::fmt::{self, Debug};
#[allow(deprecated)]
use crate::hash::{BuildHasher, Hash, Hasher, SipHasher13};
use crate::iter::FusedIterator;
use crate::ops::Index;
use crate::sys;

/// 通过二次探测和 SIMD 查找实现的 [哈希表][hash map]。
///
/// 默认情况下，`HashMap` 使用选择为提供对 HashDoS 攻击的抵抗力的哈希算法。
/// 该算法是随机播种的，并且做出了合理的努力以从主机提供的高质量，安全的随机性源生成此 seed，而不会阻塞程序。
/// 因此，seed 的随机性取决于创建 seed 时系统随机数发生器的输出质量。
/// 特别地，当系统的熵池异常低时 (例如在系统引导期间) 生成的种子可能具有较低的质量。
///
/// 当前的默认哈希算法是 SipHash 1-3，尽管它可能会在 future 的任何位置进行更改。
/// 虽然它的性能对于中等大小的键非常有竞争力，但其他哈希算法对于小键 (如整数) 和大键 (如长字符串) 的性能将优于它，尽管这些算法通常不能防止诸如 HashDoS 之类的攻击。
///
///
/// 可以使用 [`default`]，[`with_hasher`] 和 [`with_capacity_and_hasher`] 方法在每个 `HashMap` 的基础上替换哈希算法。
/// [hashing algorithms available on crates.io] 有很多替代方案。
///
/// 尽管通常可以通过使用 `#[derive(PartialEq, Eq, Hash)]` 来实现，但要求键实现 [`Eq`] 和 [`Hash`] traits。
/// 如果您自己实现这些，那么拥有以下属性非常重要：
///
/// ```text
/// k1 == k2 -> hash(k1) == hash(k2)
/// ```
///
/// 换句话说，如果两个键相等，则它们的哈希值必须相等。
///
/// 以这样一种方式修改键是一个逻辑错误，即键的哈希 (由 [`Hash`] 特征确定) 或其相等性 (由 [`Eq`] 特征确定) 在更改时发生变化在 map 上。
/// 通常只有通过 [`Cell`]，[`RefCell`]，二进制状态，I/O 或不安全代码才能实现此操作。
/// 此类逻辑错误导致的行为未指定，但会封装到观察到逻辑错误的 `HashMap` 中，不会导致未定义的行为。
/// 这可能包括 panics、不正确的结果、中止、内存泄漏和未中止。
///
/// 哈希表实现是 Google [SwissTable] 的 Rust 端口。
/// 可以在 [这里][here] 找到 SwissTable 的原始 C++ 版本，而该 [CppCon 讨论][CppCon talk] 概述了该算法的工作原理。
///
/// [hash map]: crate::collections#use-a-hashmap-when
/// [hashing algorithms available on crates.io]: https://crates.io/keywords/hasher
/// [SwissTable]: https://abseil.io/blog/20180927-swisstables
/// [here]: https://github.com/abseil/abseil-cpp/blob/master/absl/container/internal/raw_hash_set.h
/// [CppCon talk]: https://www.youtube.com/watch?v=ncHmEUmJZf4
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
///
/// // 通过类型推断，我们可以省略显式类型签名 (在本示例中为 `HashMap<String, String>`)。
/////
/// let mut book_reviews = HashMap::new();
///
/// // 复习一些书。
/// book_reviews.insert(
///     "Adventures of Huckleberry Finn".to_string(),
///     "My favorite book.".to_string(),
/// );
/// book_reviews.insert(
///     "Grimms' Fairy Tales".to_string(),
///     "Masterpiece.".to_string(),
/// );
/// book_reviews.insert(
///     "Pride and Prejudice".to_string(),
///     "Very enjoyable.".to_string(),
/// );
/// book_reviews.insert(
///     "The Adventures of Sherlock Holmes".to_string(),
///     "Eye lyked it alot.".to_string(),
/// );
///
/// // 检查一个特定的。
/// // 当集合存储拥有的值 (String) 时，仍可以使用引用 (&str) 来查询它们。
/////
/// if !book_reviews.contains_key("Les Misérables") {
///     println!("We've got {} reviews, but Les Misérables ain't one.",
///              book_reviews.len());
/// }
///
/// // 糟糕，此评论有很多拼写错误，让我们删除它。
/// book_reviews.remove("The Adventures of Sherlock Holmes");
///
/// // 查找与某些键关联的值。
/// let to_find = ["Pride and Prejudice", "Alice's Adventure in Wonderland"];
/// for &book in &to_find {
///     match book_reviews.get(book) {
///         Some(review) => println!("{book}: {review}"),
///         None => println!("{book} is unreviewed.")
///     }
/// }
///
/// // 查找某个键的值 (如果找不到该键，就会出现 panic)。
/// println!("Review for Jane: {}", book_reviews["Pride and Prejudice"]);
///
/// // 遍历所有内容。
/// for (book, review) in &book_reviews {
///     println!("{book}: \"{review}\"");
/// }
/// ```
///
/// 可以从数组初始化具有已知项列表的 `HashMap`：
///
/// ```
/// use std::collections::HashMap;
///
/// let solar_distance = HashMap::from([
///     ("Mercury", 0.4),
///     ("Venus", 0.7),
///     ("Earth", 1.0),
///     ("Mars", 1.5),
/// ]);
/// ```
///
/// `HashMap` 实现了一个 [`Entry` API](#method.entry)，它允许获取、设置、更新和删除键及其值的复杂方法:
///
/// ```
/// use std::collections::HashMap;
///
/// // 通过类型推断，我们可以省略显式类型签名 (在本示例中为 `HashMap<&str, u8>`)。
/////
/// let mut player_stats = HashMap::new();
///
/// fn random_stat_buff() -> u8 {
///     // 实际上可以在这里返回一些随机值 - 现在让我们返回一些固定值
/////
///     42
/// }
///
/// // 仅在键不存在时才插入
/// player_stats.entry("health").or_insert(100);
///
/// // 仅当一个键不存在时，才使用提供新值的函数插入该键
/////
/// player_stats.entry("defence").or_insert_with(random_stat_buff);
///
/// // 更新键，以防止键可能未被设置
/// let stat = player_stats.entry("attack").or_insert(100);
/// *stat += random_stat_buff();
///
/// // 使用就地可变的在插入之前修改条目
/// player_stats.entry("mana").and_modify(|mana| *mana += 200).or_insert(100);
/// ```
///
/// 将 `HashMap` 与自定义键类型一起使用的最简单方法是派生 [`Eq`] 和 [`Hash`]。
/// 我们还必须导出 [`PartialEq`]。
///
/// [`RefCell`]: crate::cell::RefCell
/// [`Cell`]: crate::cell::Cell
/// [`default`]: Default::default
/// [`with_hasher`]: Self::with_hasher
/// [`with_capacity_and_hasher`]: Self::with_capacity_and_hasher
///
/// ```
/// use std::collections::HashMap;
///
/// #[derive(Hash, Eq, PartialEq, Debug)]
/// struct Viking {
///     name: String,
///     country: String,
/// }
///
/// impl Viking {
///     /// 创建一个新的 Viking。
///     fn new(name: &str, country: &str) -> Viking {
///         Viking { name: name.to_string(), country: country.to_string() }
///     }
/// }
///
/// // 使用 HashMap 存储 Viking 的健康点。
/// let vikings = HashMap::from([
///     (Viking::new("Einar", "Norway"), 25),
///     (Viking::new("Olaf", "Denmark"), 24),
///     (Viking::new("Harald", "Iceland"), 12),
/// ]);
///
/// // 使用派生的实现来打印 Viking 的状态。
/// for (viking, health) in &vikings {
///     println!("{viking:?} has {health} hp");
/// }
/// ```
///
///
///
///
///
///
///
///
///
///
///
///
///
///
///
///
///

#[cfg_attr(not(test), rustc_diagnostic_item = "HashMap")]
#[stable(feature = "rust1", since = "1.0.0")]
#[rustc_insignificant_dtor]
pub struct HashMap<K, V, S = RandomState> {
    base: base::HashMap<K, V, S>,
}

impl<K, V> HashMap<K, V, RandomState> {
    /// 创建一个空的 `HashMap`。
    ///
    /// 哈希 map 最初创建时的容量为 0，因此只有在首次插入时才分配。
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// let mut map: HashMap<&str, i32> = HashMap::new();
    /// ```
    #[inline]
    #[must_use]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn new() -> HashMap<K, V, RandomState> {
        Default::default()
    }

    /// 创建一个至少具有指定容量的空 `HashMap`。
    ///
    /// 哈希 map 将能够至少保留 `capacity` 个元素而无需重新分配。
    /// 此方法允许分配比 `capacity` 更多的元素。
    /// 如果 `capacity` 为 0，则不会分配哈希 map。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// let mut map: HashMap<&str, i32> = HashMap::with_capacity(10);
    /// ```
    #[inline]
    #[must_use]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn with_capacity(capacity: usize) -> HashMap<K, V, RandomState> {
        HashMap::with_capacity_and_hasher(capacity, Default::default())
    }
}

impl<K, V, S> HashMap<K, V, S> {
    /// 创建一个空的 `HashMap`，它将使用给定的哈希生成器来哈希键。
    ///
    /// 创建的 map 具有默认的初始容量。
    ///
    /// 警告: `hash_builder` 通常是随机生成的，旨在使 HashMaps 能够抵抗导致许多冲突和非常差的性能的攻击。
    /// 使用此函数手动设置它可能会导致 DoS 攻击 vector。
    ///
    /// 传递的 `hash_builder` 应该为 HashMap 实现 [`BuildHasher`] trait 才有用，有关详细信息，请参见其文档。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use std::collections::hash_map::RandomState;
    ///
    /// let s = RandomState::new();
    /// let mut map = HashMap::with_hasher(s);
    /// map.insert(1, 2);
    /// ```
    ///
    ///
    ///
    ///
    #[inline]
    #[stable(feature = "hashmap_build_hasher", since = "1.7.0")]
    #[rustc_const_unstable(feature = "const_collections_with_hasher", issue = "102575")]
    pub const fn with_hasher(hash_builder: S) -> HashMap<K, V, S> {
        HashMap { base: base::HashMap::with_hasher(hash_builder) }
    }

    /// 创建一个至少具有指定容量的空 `HashMap`，使用 `hasher` 对键进行哈希处理。
    ///
    /// 哈希 map 将能够至少保留 `capacity` 个元素而无需重新分配。此方法允许分配比 `capacity` 更多的元素。
    /// 如果 `capacity` 为 0，则不会分配哈希 map。
    ///
    /// 警告: `hasher` 通常是随机生成的，旨在让 HashMaps 能够抵抗导致许多冲突和性能非常差的攻击。
    ///
    /// 使用此函数手动设置它可能会导致 DoS 攻击 vector。
    ///
    /// 传递的 `hasher` 应该实现 [`BuildHasher`] trait 以使 HashMap 有用，有关详细信息，请参见其文档。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use std::collections::hash_map::RandomState;
    ///
    /// let s = RandomState::new();
    /// let mut map = HashMap::with_capacity_and_hasher(10, s);
    /// map.insert(1, 2);
    /// ```
    ///
    ///
    ///
    ///
    #[inline]
    #[stable(feature = "hashmap_build_hasher", since = "1.7.0")]
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> HashMap<K, V, S> {
        HashMap { base: base::HashMap::with_capacity_and_hasher(capacity, hasher) }
    }

    /// 返回 map 无需重新分配即可容纳的元素数。
    ///
    /// 此数字是一个下限； `HashMap<K, V>` 可能可以容纳更多，但可以保证至少容纳这么多。
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// let map: HashMap<i32, i32> = HashMap::with_capacity(100);
    /// assert!(map.capacity() >= 100);
    /// ```
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn capacity(&self) -> usize {
        self.base.capacity()
    }

    /// 一个迭代器，以任意顺序访问所有键。
    /// 迭代器元素类型为 `&'a K`。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::from([
    ///     ("a", 1),
    ///     ("b", 2),
    ///     ("c", 3),
    /// ]);
    ///
    /// for key in map.keys() {
    ///     println!("{key}");
    /// }
    /// ```
    ///
    /// # Performance
    ///
    /// 在当前的实现中，迭代键需要 O(capacity) 时间而不是 O(len) 时间，因为它在内部也访问了空的 buckets。
    ///
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys { inner: self.iter() }
    }

    /// 创建一个消费迭代器，以任意顺序访问所有键。
    /// 调用后不能使用 map。
    /// 迭代器元素类型为 `K`。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::from([
    ///     ("a", 1),
    ///     ("b", 2),
    ///     ("c", 3),
    /// ]);
    ///
    /// let mut vec: Vec<&str> = map.into_keys().collect();
    /// // `IntoKeys` 迭代器以任意顺序生成键，因此必须对键进行排序以针对排序数组测试它们。
    /////
    /// vec.sort_unstable();
    /// assert_eq!(vec, ["a", "b", "c"]);
    /// ```
    ///
    /// # Performance
    ///
    /// 在当前的实现中，迭代键需要 O(capacity) 时间而不是 O(len) 时间，因为它在内部也访问了空的 buckets。
    ///
    #[inline]
    #[rustc_lint_query_instability]
    #[stable(feature = "map_into_keys_values", since = "1.54.0")]
    pub fn into_keys(self) -> IntoKeys<K, V> {
        IntoKeys { inner: self.into_iter() }
    }

    /// 一个以任意顺序访问所有值的迭代器。
    /// 迭代器元素类型为 `&'a V`。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::from([
    ///     ("a", 1),
    ///     ("b", 2),
    ///     ("c", 3),
    /// ]);
    ///
    /// for val in map.values() {
    ///     println!("{val}");
    /// }
    /// ```
    ///
    /// # Performance
    ///
    /// 在当前的实现中，迭代值需要 O(capacity) 时间而不是 O(len) 时间，因为它在内部也访问了空的 buckets。
    ///
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn values(&self) -> Values<'_, K, V> {
        Values { inner: self.iter() }
    }

    /// 一个迭代器，它以任意顺序可变地访问所有值。
    /// 迭代器元素类型为 `&'a mut V`。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut map = HashMap::from([
    ///     ("a", 1),
    ///     ("b", 2),
    ///     ("c", 3),
    /// ]);
    ///
    /// for val in map.values_mut() {
    ///     *val = *val + 10;
    /// }
    ///
    /// for val in map.values() {
    ///     println!("{val}");
    /// }
    /// ```
    ///
    /// # Performance
    ///
    /// 在当前的实现中，迭代值需要 O(capacity) 时间而不是 O(len) 时间，因为它在内部也访问了空的 buckets。
    ///
    #[stable(feature = "map_values_mut", since = "1.10.0")]
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        ValuesMut { inner: self.iter_mut() }
    }

    /// 创建一个消费迭代器，以任意顺序访问所有值。
    /// 调用后不能使用 map。
    /// 迭代器元素类型为 `V`。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::from([
    ///     ("a", 1),
    ///     ("b", 2),
    ///     ("c", 3),
    /// ]);
    ///
    /// let mut vec: Vec<i32> = map.into_values().collect();
    /// // `IntoValues` 迭代器以任意顺序生成值，因此必须对这些值进行排序以针对已排序数组对其进行测试。
    /////
    /// vec.sort_unstable();
    /// assert_eq!(vec, [1, 2, 3]);
    /// ```
    ///
    /// # Performance
    ///
    /// 在当前的实现中，迭代值需要 O(capacity) 时间而不是 O(len) 时间，因为它在内部也访问了空的 buckets。
    ///
    #[inline]
    #[rustc_lint_query_instability]
    #[stable(feature = "map_into_keys_values", since = "1.54.0")]
    pub fn into_values(self) -> IntoValues<K, V> {
        IntoValues { inner: self.into_iter() }
    }

    /// 一个迭代器，以任意顺序访问所有键值对。
    /// 迭代器元素类型为 `(&'a K, &'a V)`。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::from([
    ///     ("a", 1),
    ///     ("b", 2),
    ///     ("c", 3),
    /// ]);
    ///
    /// for (key, val) in map.iter() {
    ///     println!("key: {key} val: {val}");
    /// }
    /// ```
    ///
    /// # Performance
    ///
    /// 在当前实现中，迭代 map 需要 O(capacity) 时间而不是 O(len) 时间，因为它在内部也访问了空的 buckets。
    ///
    #[rustc_lint_query_instability]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter { base: self.base.iter() }
    }

    /// 一个迭代器，以任意顺序访问所有键值对，并且对值进行可变引用。
    /// 迭代器元素类型为 `(&'a K, &'a mut V)`。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut map = HashMap::from([
    ///     ("a", 1),
    ///     ("b", 2),
    ///     ("c", 3),
    /// ]);
    ///
    /// // 更新所有值
    /// for (_, val) in map.iter_mut() {
    ///     *val *= 2;
    /// }
    ///
    /// for (key, val) in &map {
    ///     println!("key: {key} val: {val}");
    /// }
    /// ```
    ///
    /// # Performance
    ///
    /// 在当前实现中，迭代 map 需要 O(capacity) 时间而不是 O(len) 时间，因为它在内部也访问了空的 buckets。
    ///
    ///
    #[rustc_lint_query_instability]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut { base: self.base.iter_mut() }
    }

    /// 返回 map 中的元素数。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut a = HashMap::new();
    /// assert_eq!(a.len(), 0);
    /// a.insert(1, "a");
    /// assert_eq!(a.len(), 1);
    /// ```
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn len(&self) -> usize {
        self.base.len()
    }

    /// 如果 map 不包含任何元素，则返回 `true`。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut a = HashMap::new();
    /// assert!(a.is_empty());
    /// a.insert(1, "a");
    /// assert!(!a.is_empty());
    /// ```
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn is_empty(&self) -> bool {
        self.base.is_empty()
    }

    /// 清除 map，将所有键值对作为迭代器返回。保留分配的内存以供重用。
    ///
    /// 如果返回的迭代器在被完全消耗之前被丢弃，则丢弃剩余的键值对
    /// 返回的迭代器在 map 上保留一个错误借用以优化其实现。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut a = HashMap::new();
    /// a.insert(1, "a");
    /// a.insert(2, "b");
    ///
    /// for (k, v) in a.drain().take(1) {
    ///     assert!(k == 1 || k == 2);
    ///     assert!(v == "a" || v == "b");
    /// }
    ///
    /// assert!(a.is_empty());
    /// ```
    ///
    ///
    #[inline]
    #[rustc_lint_query_instability]
    #[stable(feature = "drain", since = "1.6.0")]
    pub fn drain(&mut self) -> Drain<'_, K, V> {
        Drain { base: self.base.drain() }
    }

    /// 创建一个迭代器，该迭代器使用闭包确定是否应删除元素。
    ///
    /// 如果闭包返回 true，则将元素从 map 中移除并产生。
    /// 如果闭包返回 false 或 panics，则该元素保留在 map 中，并且不会产生。
    ///
    /// 请注意，无论选择保留还是删除 `drain_filter`，您都可以对过滤器闭包中的每个值进行可变的。
    ///
    /// 如果迭代器仅被部分消耗或根本没有被消耗，则其余所有元素仍将受到闭包的处理，如果返回 true，则将其删除并丢弃。
    ///
    /// 如果在闭包中出现 panic，或者在丢弃元素时发生 panic，或者 `DrainFilter` 值泄漏，将有多少个元素受到该闭包的影响，这是不确定的。
    ///
    ///
    /// # Examples
    ///
    /// 将 map 分为偶数和奇数键，重新使用原始的 map：
    ///
    /// ```
    /// #![feature(hash_drain_filter)]
    /// use std::collections::HashMap;
    ///
    /// let mut map: HashMap<i32, i32> = (0..8).map(|x| (x, x)).collect();
    /// let drained: HashMap<i32, i32> = map.drain_filter(|k, _v| k % 2 == 0).collect();
    ///
    /// let mut evens = drained.keys().copied().collect::<Vec<_>>();
    /// let mut odds = map.keys().copied().collect::<Vec<_>>();
    /// evens.sort();
    /// odds.sort();
    ///
    /// assert_eq!(evens, vec![0, 2, 4, 6]);
    /// assert_eq!(odds, vec![1, 3, 5, 7]);
    /// ```
    ///
    ///
    ///
    ///
    #[inline]
    #[rustc_lint_query_instability]
    #[unstable(feature = "hash_drain_filter", issue = "59618")]
    pub fn drain_filter<F>(&mut self, pred: F) -> DrainFilter<'_, K, V, F>
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        DrainFilter { base: self.base.drain_filter(pred) }
    }

    /// 仅保留谓词指定的元素。
    ///
    /// 换句话说，删除所有 `f(&k, &mut v)` 返回 `false` 的 `(k, v)` 对。
    /// 元素以未排序 (和未指定) 的顺序访问。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut map: HashMap<i32, i32> = (0..8).map(|x| (x, x*10)).collect();
    /// map.retain(|&k, _| k % 2 == 0);
    /// assert_eq!(map.len(), 4);
    /// ```
    ///
    /// # Performance
    ///
    /// 在当前的实现中，这个操作需要 O(capacity) 时间而不是 O(len)，因为它在内部也访问了空的 buckets。
    ///
    #[inline]
    #[rustc_lint_query_instability]
    #[stable(feature = "retain_hash_collection", since = "1.18.0")]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        self.base.retain(f)
    }

    /// 清除 map，删除所有键值对。
    /// 保留分配的内存以供重用。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut a = HashMap::new();
    /// a.insert(1, "a");
    /// a.clear();
    /// assert!(a.is_empty());
    /// ```
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn clear(&mut self) {
        self.base.clear();
    }

    /// 返回 map 的 [`BuildHasher`] 的引用。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use std::collections::hash_map::RandomState;
    ///
    /// let hasher = RandomState::new();
    /// let map: HashMap<i32, i32> = HashMap::with_hasher(hasher);
    /// let hasher: &RandomState = map.hasher();
    /// ```
    #[inline]
    #[stable(feature = "hashmap_public_hasher", since = "1.9.0")]
    pub fn hasher(&self) -> &S {
        self.base.hasher()
    }
}

impl<K, V, S> HashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    /// 保留至少 `additional` 个要插入 `HashMap` 中的更多元素的容量。集合可以保留更多空间来推测性地避免频繁的重新分配。
    ///
    /// 调用 `reserve` 后，容量将大于或等于 `self.len() + additional`。
    /// 如果容量已经足够，则不执行任何操作。
    ///
    /// # Panics
    ///
    /// 如果新的分配大小溢出 [`usize`]，就会出现 panics。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// let mut map: HashMap<&str, i32> = HashMap::new();
    /// map.reserve(10);
    /// ```
    ///
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn reserve(&mut self, additional: usize) {
        self.base.reserve(additional)
    }

    /// 尝试为要插入到 `HashMap` 中的至少 `additional` 更多元素保留容量。集合可以保留更多空间来推测性地避免频繁的重新分配。
    ///
    /// 调用 `try_reserve` 后，如果返回 `Ok(())`，容量将大于等于 `self.len() + additional`。
    /// 如果容量已经足够，则不执行任何操作。
    ///
    /// # Errors
    ///
    /// 如果容量溢出，或者分配器报告失败，则返回错误。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut map: HashMap<&str, isize> = HashMap::new();
    /// map.try_reserve(10).expect("why is the test harness OOMing on a handful of bytes?");
    /// ```
    ///
    ///
    ///
    #[inline]
    #[stable(feature = "try_reserve", since = "1.57.0")]
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.base.try_reserve(additional).map_err(map_try_reserve_error)
    }

    /// 尽可能缩小 map 的容量。
    /// 它会在保持内部规则的同时尽可能地丢弃，并可能根据调整大小策略留出一些空间。
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut map: HashMap<i32, i32> = HashMap::with_capacity(100);
    /// map.insert(1, 2);
    /// map.insert(3, 4);
    /// assert!(map.capacity() >= 100);
    /// map.shrink_to_fit();
    /// assert!(map.capacity() >= 2);
    /// ```
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn shrink_to_fit(&mut self) {
        self.base.shrink_to_fit();
    }

    /// 降低 map 的容量。
    /// 它将降低不低于提供的限制，同时保持内部规则，并可能根据调整大小策略留下一些空间。
    ///
    ///
    /// 如果当前容量小于下限，则为无操作。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut map: HashMap<i32, i32> = HashMap::with_capacity(100);
    /// map.insert(1, 2);
    /// map.insert(3, 4);
    /// assert!(map.capacity() >= 100);
    /// map.shrink_to(10);
    /// assert!(map.capacity() >= 10);
    /// map.shrink_to(0);
    /// assert!(map.capacity() >= 2);
    /// ```
    #[inline]
    #[stable(feature = "shrink_to", since = "1.56.0")]
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.base.shrink_to(min_capacity);
    }

    /// 在 map 中获取给定键的对应项，以进行就地操作。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut letters = HashMap::new();
    ///
    /// for ch in "a short treatise on fungi".chars() {
    ///     letters.entry(ch).and_modify(|counter| *counter += 1).or_insert(1);
    /// }
    ///
    /// assert_eq!(letters[&'s'], 2);
    /// assert_eq!(letters[&'t'], 3);
    /// assert_eq!(letters[&'u'], 1);
    /// assert_eq!(letters.get(&'y'), None);
    /// ```
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        map_entry(self.base.rustc_entry(key))
    }

    /// 返回与键对应的值的引用。
    ///
    /// 键可以是 map 键类型的任何借用形式，但是借用形式上的 [`Hash`] 和 [`Eq`] 必须与键的类型匹配。
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut map = HashMap::new();
    /// map.insert(1, "a");
    /// assert_eq!(map.get(&1), Some(&"a"));
    /// assert_eq!(map.get(&2), None);
    /// ```
    ///
    #[stable(feature = "rust1", since = "1.0.0")]
    #[inline]
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.base.get(k)
    }

    /// 返回与提供的键相对应的键值对。
    ///
    /// 提供的键可以是 map 的键类型的任何借用形式，但是借用形式上的 [`Hash`] 和 [`Eq`] 必须与该键的类型匹配。
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut map = HashMap::new();
    /// map.insert(1, "a");
    /// assert_eq!(map.get_key_value(&1), Some((&1, &"a")));
    /// assert_eq!(map.get_key_value(&2), None);
    /// ```
    ///
    #[inline]
    #[stable(feature = "map_get_key_value", since = "1.40.0")]
    pub fn get_key_value<Q: ?Sized>(&self, k: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.base.get_key_value(k)
    }

    /// 尝试立即获取 map 中的 `N` 值的异常引用。
    ///
    /// 返回一个长度为 `N` 的数组，其中包含每个查询的结果。
    /// 为了稳健性，最多将一个错误引用返回为任何值。
    /// 如果任何键重复或丢失，将返回 `None`。
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(map_many_mut)]
    /// use std::collections::HashMap;
    ///
    /// let mut libraries = HashMap::new();
    /// libraries.insert("Bodleian Library".to_string(), 1602);
    /// libraries.insert("Athenæum".to_string(), 1807);
    /// libraries.insert("Herzogin-Anna-Amalia-Bibliothek".to_string(), 1691);
    /// libraries.insert("Library of Congress".to_string(), 1800);
    ///
    /// let got = libraries.get_many_mut([
    ///     "Athenæum",
    ///     "Library of Congress",
    /// ]);
    /// assert_eq!(
    ///     got,
    ///     Some([
    ///         &mut 1807,
    ///         &mut 1800,
    ///     ]),
    /// );
    ///
    /// // 缺少键会导致 None
    /// let got = libraries.get_many_mut([
    ///     "Athenæum",
    ///     "New York Public Library",
    /// ]);
    /// assert_eq!(got, None);
    ///
    /// // 重复的键会导致 None
    /// let got = libraries.get_many_mut([
    ///     "Athenæum",
    ///     "Athenæum",
    /// ]);
    /// assert_eq!(got, None);
    /// ```
    #[inline]
    #[unstable(feature = "map_many_mut", issue = "97601")]
    pub fn get_many_mut<Q: ?Sized, const N: usize>(&mut self, ks: [&Q; N]) -> Option<[&'_ mut V; N]>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.base.get_many_mut(ks)
    }

    /// 尝试立即获取 X 引用到 map 中的 `N` 值，而不验证这些值是否唯一。
    ///
    ///
    /// 返回一个长度为 `N` 的数组，其中包含每个查询的结果。
    /// 如果缺少任何键，将返回 `None`。
    ///
    /// 有关安全的替代方案，请参见 [`get_many_mut`](Self::get_many_mut)。
    ///
    /// # Safety
    ///
    /// 使用重叠键调用此方法是 *[undefined behavior]*，即使不使用生成的引用。
    ///
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(map_many_mut)]
    /// use std::collections::HashMap;
    ///
    /// let mut libraries = HashMap::new();
    /// libraries.insert("Bodleian Library".to_string(), 1602);
    /// libraries.insert("Athenæum".to_string(), 1807);
    /// libraries.insert("Herzogin-Anna-Amalia-Bibliothek".to_string(), 1691);
    /// libraries.insert("Library of Congress".to_string(), 1800);
    ///
    /// let got = libraries.get_many_mut([
    ///     "Athenæum",
    ///     "Library of Congress",
    /// ]);
    /// assert_eq!(
    ///     got,
    ///     Some([
    ///         &mut 1807,
    ///         &mut 1800,
    ///     ]),
    /// );
    ///
    /// // 缺少键会导致 None
    /// let got = libraries.get_many_mut([
    ///     "Athenæum",
    ///     "New York Public Library",
    /// ]);
    /// assert_eq!(got, None);
    /// ```
    ///
    #[inline]
    #[unstable(feature = "map_many_mut", issue = "97601")]
    pub unsafe fn get_many_unchecked_mut<Q: ?Sized, const N: usize>(
        &mut self,
        ks: [&Q; N],
    ) -> Option<[&'_ mut V; N]>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.base.get_many_unchecked_mut(ks)
    }

    /// 如果 map 包含指定键的值，则返回 `true`。
    ///
    /// 键可以是 map 键类型的任何借用形式，但是借用形式上的 [`Hash`] 和 [`Eq`] 必须与键的类型匹配。
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut map = HashMap::new();
    /// map.insert(1, "a");
    /// assert_eq!(map.contains_key(&1), true);
    /// assert_eq!(map.contains_key(&2), false);
    /// ```
    ///
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.base.contains_key(k)
    }

    /// 返回与键对应的值的可变引用。
    ///
    /// 键可以是 map 键类型的任何借用形式，但是借用形式上的 [`Hash`] 和 [`Eq`] 必须与键的类型匹配。
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut map = HashMap::new();
    /// map.insert(1, "a");
    /// if let Some(x) = map.get_mut(&1) {
    ///     *x = "b";
    /// }
    /// assert_eq!(map[&1], "b");
    /// ```
    ///
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.base.get_mut(k)
    }

    /// 将键值对插入 map。
    ///
    /// 如果 map 不存在此键，则返回 [`None`]。
    ///
    /// 如果 map 确实存在此键，则更新值，并返回旧值。
    /// 但是，键不会更新。对于不能相同的 `==` 类型来说，这一点很重要。
    ///
    /// 有关更多信息，请参见 [模块级文档][module-level documentation]。
    ///
    /// [module-level documentation]: crate::collections#insert-and-complex-keys
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut map = HashMap::new();
    /// assert_eq!(map.insert(37, "a"), None);
    /// assert_eq!(map.is_empty(), false);
    ///
    /// map.insert(37, "b");
    /// assert_eq!(map.insert(37, "c"), Some("b"));
    /// assert_eq!(map[&37], "c");
    /// ```
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.base.insert(k, v)
    }

    /// 尝试将键值对插入到 map 中，并向条目中的值返回变量引用。
    ///
    /// 如果 map 已经存在此键，则不进行任何更新，并返回包含占用项和值的错误。
    ///
    ///
    /// # Examples
    ///
    /// 基本用法：
    ///
    /// ```
    /// #![feature(map_try_insert)]
    ///
    /// use std::collections::HashMap;
    ///
    /// let mut map = HashMap::new();
    /// assert_eq!(map.try_insert(37, "a").unwrap(), &"a");
    ///
    /// let err = map.try_insert(37, "b").unwrap_err();
    /// assert_eq!(err.entry.key(), &37);
    /// assert_eq!(err.entry.get(), &"a");
    /// assert_eq!(err.value, "b");
    /// ```
    ///
    #[unstable(feature = "map_try_insert", issue = "82766")]
    pub fn try_insert(&mut self, key: K, value: V) -> Result<&mut V, OccupiedError<'_, K, V>> {
        match self.entry(key) {
            Occupied(entry) => Err(OccupiedError { entry, value }),
            Vacant(entry) => Ok(entry.insert(value)),
        }
    }

    /// 从 map 中删除一个键，如果该键以前在 map 中，则返回该键的值。
    ///
    /// 键可以是 map 键类型的任何借用形式，但是借用形式上的 [`Hash`] 和 [`Eq`] 必须与键的类型匹配。
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut map = HashMap::new();
    /// map.insert(1, "a");
    /// assert_eq!(map.remove(&1), Some("a"));
    /// assert_eq!(map.remove(&1), None);
    /// ```
    ///
    ///
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn remove<Q: ?Sized>(&mut self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.base.remove(k)
    }

    /// 从 map 中删除一个键，如果该键以前在 map 中，则返回存储的键和值。
    ///
    /// 键可以是 map 键类型的任何借用形式，但是借用形式上的 [`Hash`] 和 [`Eq`] 必须与键的类型匹配。
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// # fn main() {
    /// let mut map = HashMap::new();
    /// map.insert(1, "a");
    /// assert_eq!(map.remove_entry(&1), Some((1, "a")));
    /// assert_eq!(map.remove(&1), None);
    /// # }
    /// ```
    ///
    ///
    #[inline]
    #[stable(feature = "hash_map_remove_entry", since = "1.27.0")]
    pub fn remove_entry<Q: ?Sized>(&mut self, k: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.base.remove_entry(k)
    }
}

impl<K, V, S> HashMap<K, V, S>
where
    S: BuildHasher,
{
    /// 为 HashMap 创建原始条目构建器。
    ///
    /// 原始条目为搜索和操作 map 提供了最低级别的控制。必须使用哈希将其手动初始化，然后进行手动搜索。
    /// 此后，插入空条目仍然需要提供一个拥有的键。
    ///
    /// 原始条目对于以下特殊情况很有用：
    ///
    /// * 哈希记忆
    /// * 推迟创建拥有的键，直到知道它是必需的为止
    /// * 使用不适用于借用 trait 的搜索键
    /// * 在不使用 newtype 包装器的情况下使用自定义比较逻辑
    ///
    /// 因为原始条目提供了更多的灵活控制，所以将 HashMap 置于不一致状态要容易得多，这虽然具有内存安全性，但会导致 map 产生看似随机的结果。
    /// 如果可能，应首选更高级别且更简单的 API，例如 `entry`。
    ///
    /// 特别是，用于初始化原始条目的哈希必须仍然与最终存储在条目中的键的哈希保持一致。
    /// 这是因为 HashMap 的实现在调整大小时可能需要重新计算哈希，此时只有键可用。
    ///
    /// 原始条目为变量提供了可变的访问权限。
    /// 这不能用于修改键的比较或散列方式，因为映射不会重新评估键应该去的位置，这意味着如果它们的位置不反映它们的状态，它们可能会丢失。
    ///
    /// 例如，如果您更改一个键以使 map 现在包含比较相等的键，则搜索可能会开始不规律地进行，两个键彼此相互掩盖。
    /// 实现可以自由地假设不会发生这种情况 (在内存安全性的范围内)。
    ///
    ///
    ///
    ///
    ///
    ///
    ///
    ///
    #[inline]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn raw_entry_mut(&mut self) -> RawEntryBuilderMut<'_, K, V, S> {
        RawEntryBuilderMut { map: self }
    }

    /// 为 HashMap 创建一个原始的不可变条目构建器。
    ///
    /// 原始条目为搜索和操作 map 提供了最低级别的控制。
    /// 必须使用哈希将其手动初始化，然后进行手动搜索。
    ///
    /// 这对于
    /// * 哈希记忆
    /// * 使用不适用于借用 trait 的搜索键
    /// * 在不使用 newtype 包装器的情况下使用自定义比较逻辑
    ///
    /// 除非您处于这种情况下，否则应首选更高级别且更简单的 API，例如 `get`。
    ///
    ///
    /// 不可变的原始条目用途非常有限； 您可能需要 `raw_entry_mut`。
    ///
    #[inline]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn raw_entry(&self) -> RawEntryBuilder<'_, K, V, S> {
        RawEntryBuilder { map: self }
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<K, V, S> Clone for HashMap<K, V, S>
where
    K: Clone,
    V: Clone,
    S: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self { base: self.base.clone() }
    }

    #[inline]
    fn clone_from(&mut self, other: &Self) {
        self.base.clone_from(&other.base);
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<K, V, S> PartialEq for HashMap<K, V, S>
where
    K: Eq + Hash,
    V: PartialEq,
    S: BuildHasher,
{
    fn eq(&self, other: &HashMap<K, V, S>) -> bool {
        if self.len() != other.len() {
            return false;
        }

        self.iter().all(|(key, value)| other.get(key).map_or(false, |v| *value == *v))
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<K, V, S> Eq for HashMap<K, V, S>
where
    K: Eq + Hash,
    V: Eq,
    S: BuildHasher,
{
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<K, V, S> Debug for HashMap<K, V, S>
where
    K: Debug,
    V: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<K, V, S> Default for HashMap<K, V, S>
where
    S: Default,
{
    /// 创建一个空的 `HashMap<K, V, S>`，其哈希值为 `Default`。
    #[inline]
    fn default() -> HashMap<K, V, S> {
        HashMap::with_hasher(Default::default())
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<K, Q: ?Sized, V, S> Index<&Q> for HashMap<K, V, S>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash,
    S: BuildHasher,
{
    type Output = V;

    /// 返回与提供的键对应的值的引用。
    ///
    /// # Panics
    ///
    /// 如果键不在 `HashMap` 中，就会出现 panic。
    #[inline]
    fn index(&self, key: &Q) -> &V {
        self.get(key).expect("no entry found for key")
    }
}

#[stable(feature = "std_collections_from_array", since = "1.56.0")]
// Note: 作为目前构建 HashMap 的最方便的内置方式，这个函数的简单用法不能*要求*用户提供类型注解以推断第三个类型参数 (哈希参数，通常为 "S") .
//
// 为此，这个 impl 使用 RandomState 作为 S 的具体类型来定义，而不是 `S: BuildHasher + Default` 上的泛型。
// 预计想要指定哈希器的用户将手动使用 `with_capacity_and_hasher`。
// 如果类型参数默认值适用于 impls，并且如果类型参数默认值可以与 const 泛型混合，那么这也许可以推广。
// 另请参见 HashSet 上的等效实现。
//
//
//
//
//
//
impl<K, V, const N: usize> From<[(K, V); N]> for HashMap<K, V, RandomState>
where
    K: Eq + Hash,
{
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let map1 = HashMap::from([(1, 2), (3, 4)]);
    /// let map2: HashMap<_, _> = [(1, 2), (3, 4)].into();
    /// assert_eq!(map1, map2);
    /// ```
    fn from(arr: [(K, V); N]) -> Self {
        Self::from_iter(arr)
    }
}

/// `HashMap` 条目上的迭代器。
///
/// 该 `struct` 是通过 [`HashMap`] 上的 [`iter`] 方法创建的。
/// 有关更多信息，请参见其文档。
///
/// [`iter`]: HashMap::iter
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
///
/// let map = HashMap::from([
///     ("a", 1),
/// ]);
/// let iter = map.iter();
/// ```
#[stable(feature = "rust1", since = "1.0.0")]
pub struct Iter<'a, K: 'a, V: 'a> {
    base: base::Iter<'a, K, V>,
}

// FIXME(#26925) 删除以支持 `#[derive(Clone)]`
#[stable(feature = "rust1", since = "1.0.0")]
impl<K, V> Clone for Iter<'_, K, V> {
    #[inline]
    fn clone(&self) -> Self {
        Iter { base: self.base.clone() }
    }
}

#[stable(feature = "std_debug", since = "1.16.0")]
impl<K: Debug, V: Debug> fmt::Debug for Iter<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// `HashMap` 条目上的可变迭代器。
///
/// 该 `struct` 是通过 [`HashMap`] 上的 [`iter_mut`] 方法创建的。
/// 有关更多信息，请参见其文档。
///
/// [`iter_mut`]: HashMap::iter_mut
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
///
/// let mut map = HashMap::from([
///     ("a", 1),
/// ]);
/// let iter = map.iter_mut();
/// ```
#[stable(feature = "rust1", since = "1.0.0")]
pub struct IterMut<'a, K: 'a, V: 'a> {
    base: base::IterMut<'a, K, V>,
}

impl<'a, K, V> IterMut<'a, K, V> {
    /// 返回其余项上的迭代器。
    #[inline]
    pub(super) fn iter(&self) -> Iter<'_, K, V> {
        Iter { base: self.base.rustc_iter() }
    }
}

/// `HashMap` 条目上的所有者迭代器。
///
/// 这个 `struct` 是通过 [`HashMap`] 上的 [`into_iter`] 方法创建的 (由 [`IntoIterator`] trait 提供)。
/// 有关更多信息，请参见其文档。
///
/// [`into_iter`]: IntoIterator::into_iter
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
///
/// let map = HashMap::from([
///     ("a", 1),
/// ]);
/// let iter = map.into_iter();
/// ```
#[stable(feature = "rust1", since = "1.0.0")]
pub struct IntoIter<K, V> {
    base: base::IntoIter<K, V>,
}

impl<K, V> IntoIter<K, V> {
    /// 返回其余项上的迭代器。
    #[inline]
    pub(super) fn iter(&self) -> Iter<'_, K, V> {
        Iter { base: self.base.rustc_iter() }
    }
}

/// `HashMap` 的键上的迭代器。
///
/// 该 `struct` 是通过 [`HashMap`] 上的 [`keys`] 方法创建的。
/// 有关更多信息，请参见其文档。
///
/// [`keys`]: HashMap::keys
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
///
/// let map = HashMap::from([
///     ("a", 1),
/// ]);
/// let iter_keys = map.keys();
/// ```
#[stable(feature = "rust1", since = "1.0.0")]
pub struct Keys<'a, K: 'a, V: 'a> {
    inner: Iter<'a, K, V>,
}

// FIXME(#26925) 删除以支持 `#[derive(Clone)]`
#[stable(feature = "rust1", since = "1.0.0")]
impl<K, V> Clone for Keys<'_, K, V> {
    #[inline]
    fn clone(&self) -> Self {
        Keys { inner: self.inner.clone() }
    }
}

#[stable(feature = "std_debug", since = "1.16.0")]
impl<K: Debug, V> fmt::Debug for Keys<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// `HashMap` 值的迭代器。
///
/// 该 `struct` 是通过 [`HashMap`] 上的 [`values`] 方法创建的。
/// 有关更多信息，请参见其文档。
///
/// [`values`]: HashMap::values
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
///
/// let map = HashMap::from([
///     ("a", 1),
/// ]);
/// let iter_values = map.values();
/// ```
#[stable(feature = "rust1", since = "1.0.0")]
pub struct Values<'a, K: 'a, V: 'a> {
    inner: Iter<'a, K, V>,
}

// FIXME(#26925) 删除以支持 `#[derive(Clone)]`
#[stable(feature = "rust1", since = "1.0.0")]
impl<K, V> Clone for Values<'_, K, V> {
    #[inline]
    fn clone(&self) -> Self {
        Values { inner: self.inner.clone() }
    }
}

#[stable(feature = "std_debug", since = "1.16.0")]
impl<K, V: Debug> fmt::Debug for Values<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// `HashMap` 条目上的 draining 迭代器。
///
/// 该 `struct` 是通过 [`HashMap`] 上的 [`drain`] 方法创建的。
/// 有关更多信息，请参见其文档。
///
/// [`drain`]: HashMap::drain
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
///
/// let mut map = HashMap::from([
///     ("a", 1),
/// ]);
/// let iter = map.drain();
/// ```
#[stable(feature = "drain", since = "1.6.0")]
pub struct Drain<'a, K: 'a, V: 'a> {
    base: base::Drain<'a, K, V>,
}

impl<'a, K, V> Drain<'a, K, V> {
    /// 返回其余项上的迭代器。
    #[inline]
    pub(super) fn iter(&self) -> Iter<'_, K, V> {
        Iter { base: self.base.rustc_iter() }
    }
}

/// draining，对 `HashMap` 的条目进行过滤迭代器。
///
/// 该 `struct` 是通过 [`HashMap`] 上的 [`drain_filter`] 方法创建的。
///
/// [`drain_filter`]: HashMap::drain_filter
///
/// # Example
///
/// ```
/// #![feature(hash_drain_filter)]
///
/// use std::collections::HashMap;
///
/// let mut map = HashMap::from([
///     ("a", 1),
/// ]);
/// let iter = map.drain_filter(|_k, v| *v % 2 == 0);
/// ```
#[unstable(feature = "hash_drain_filter", issue = "59618")]
pub struct DrainFilter<'a, K, V, F>
where
    F: FnMut(&K, &mut V) -> bool,
{
    base: base::DrainFilter<'a, K, V, F>,
}

/// `HashMap` 的值上的可变迭代器。
///
/// 该 `struct` 是通过 [`HashMap`] 上的 [`values_mut`] 方法创建的。
/// 有关更多信息，请参见其文档。
///
/// [`values_mut`]: HashMap::values_mut
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
///
/// let mut map = HashMap::from([
///     ("a", 1),
/// ]);
/// let iter_values = map.values_mut();
/// ```
#[stable(feature = "map_values_mut", since = "1.10.0")]
pub struct ValuesMut<'a, K: 'a, V: 'a> {
    inner: IterMut<'a, K, V>,
}

/// `HashMap` 的键上的拥有的迭代器。
///
/// 该 `struct` 是通过 [`HashMap`] 上的 [`into_keys`] 方法创建的。
/// 有关更多信息，请参见其文档。
///
/// [`into_keys`]: HashMap::into_keys
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
///
/// let map = HashMap::from([
///     ("a", 1),
/// ]);
/// let iter_keys = map.into_keys();
/// ```
#[stable(feature = "map_into_keys_values", since = "1.54.0")]
pub struct IntoKeys<K, V> {
    inner: IntoIter<K, V>,
}

/// `HashMap` 的值上的拥有的迭代器。
///
/// 该 `struct` 是通过 [`HashMap`] 上的 [`into_values`] 方法创建的。
/// 有关更多信息，请参见其文档。
///
/// [`into_values`]: HashMap::into_values
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
///
/// let map = HashMap::from([
///     ("a", 1),
/// ]);
/// let iter_keys = map.into_values();
/// ```
#[stable(feature = "map_into_keys_values", since = "1.54.0")]
pub struct IntoValues<K, V> {
    inner: IntoIter<K, V>,
}

/// 一个用于计算 HashMap 中的键值对将存储在哪里的构建器。
///
/// 有关用法示例，请参见 [`HashMap::raw_entry_mut`] 文档。
#[unstable(feature = "hash_raw_entry", issue = "56167")]
pub struct RawEntryBuilderMut<'a, K: 'a, V: 'a, S: 'a> {
    map: &'a mut HashMap<K, V, S>,
}

/// map 中单个条目的视图，该条目可能是空的或被已占用。
///
/// 这是 [`Entry`] 的较低版本。
///
/// 该 `enum` 是通过 [`HashMap`] 上的 [`raw_entry_mut`] 方法构造的，然后调用该 [`RawEntryBuilderMut`] 的方法之一。
///
///
/// [`raw_entry_mut`]: HashMap::raw_entry_mut
#[unstable(feature = "hash_raw_entry", issue = "56167")]
pub enum RawEntryMut<'a, K: 'a, V: 'a, S: 'a> {
    /// 一个被占用的条目。
    Occupied(RawOccupiedEntryMut<'a, K, V, S>),
    /// 一个空的条目。
    Vacant(RawVacantEntryMut<'a, K, V, S>),
}

/// `HashMap` 中已占用条目的视图。
/// 它是 [`RawEntryMut`] 枚举的一部分。
#[unstable(feature = "hash_raw_entry", issue = "56167")]
pub struct RawOccupiedEntryMut<'a, K: 'a, V: 'a, S: 'a> {
    base: base::RawOccupiedEntryMut<'a, K, V, S>,
}

/// `HashMap` 中空闲条目的视图。
/// 它是 [`RawEntryMut`] 枚举的一部分。
#[unstable(feature = "hash_raw_entry", issue = "56167")]
pub struct RawVacantEntryMut<'a, K: 'a, V: 'a, S: 'a> {
    base: base::RawVacantEntryMut<'a, K, V, S>,
}

/// 一个用于计算 HashMap 中的键值对将存储在哪里的构建器。
///
/// 有关用法示例，请参见 [`HashMap::raw_entry`] 文档。
#[unstable(feature = "hash_raw_entry", issue = "56167")]
pub struct RawEntryBuilder<'a, K: 'a, V: 'a, S: 'a> {
    map: &'a HashMap<K, V, S>,
}

impl<'a, K, V, S> RawEntryBuilderMut<'a, K, V, S>
where
    S: BuildHasher,
{
    /// 从给定的键创建一个 `RawEntryMut`。
    #[inline]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn from_key<Q: ?Sized>(self, k: &Q) -> RawEntryMut<'a, K, V, S>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        map_raw_entry(self.map.base.raw_entry_mut().from_key(k))
    }

    /// 根据给定的键及其哈希值创建 `RawEntryMut`。
    #[inline]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn from_key_hashed_nocheck<Q: ?Sized>(self, hash: u64, k: &Q) -> RawEntryMut<'a, K, V, S>
    where
        K: Borrow<Q>,
        Q: Eq,
    {
        map_raw_entry(self.map.base.raw_entry_mut().from_key_hashed_nocheck(hash, k))
    }

    /// 从给定的哈希创建 `RawEntryMut`。
    #[inline]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn from_hash<F>(self, hash: u64, is_match: F) -> RawEntryMut<'a, K, V, S>
    where
        for<'b> F: FnMut(&'b K) -> bool,
    {
        map_raw_entry(self.map.base.raw_entry_mut().from_hash(hash, is_match))
    }
}

impl<'a, K, V, S> RawEntryBuilder<'a, K, V, S>
where
    S: BuildHasher,
{
    /// 通过键访问条目。
    #[inline]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn from_key<Q: ?Sized>(self, k: &Q) -> Option<(&'a K, &'a V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.map.base.raw_entry().from_key(k)
    }

    /// 通过键及其哈希值访问条目。
    #[inline]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn from_key_hashed_nocheck<Q: ?Sized>(self, hash: u64, k: &Q) -> Option<(&'a K, &'a V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.map.base.raw_entry().from_key_hashed_nocheck(hash, k)
    }

    /// 通过哈希访问条目。
    #[inline]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn from_hash<F>(self, hash: u64, is_match: F) -> Option<(&'a K, &'a V)>
    where
        F: FnMut(&K) -> bool,
    {
        self.map.base.raw_entry().from_hash(hash, is_match)
    }
}

impl<'a, K, V, S> RawEntryMut<'a, K, V, S> {
    /// 通过插入默认值 (如果为空) 来确保值在条目中，并向条目中的键和值返回可变引用。
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(hash_raw_entry)]
    /// use std::collections::HashMap;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    ///
    /// map.raw_entry_mut().from_key("poneyland").or_insert("poneyland", 3);
    /// assert_eq!(map["poneyland"], 3);
    ///
    /// *map.raw_entry_mut().from_key("poneyland").or_insert("poneyland", 10).1 *= 2;
    /// assert_eq!(map["poneyland"], 6);
    /// ```
    #[inline]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn or_insert(self, default_key: K, default_val: V) -> (&'a mut K, &'a mut V)
    where
        K: Hash,
        S: BuildHasher,
    {
        match self {
            RawEntryMut::Occupied(entry) => entry.into_key_value(),
            RawEntryMut::Vacant(entry) => entry.insert(default_key, default_val),
        }
    }

    /// 通过插入默认函数 (如果为空) 的结果来确保值在条目中，并在条目中的键和值上返回变量引用。
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(hash_raw_entry)]
    /// use std::collections::HashMap;
    ///
    /// let mut map: HashMap<&str, String> = HashMap::new();
    ///
    /// map.raw_entry_mut().from_key("poneyland").or_insert_with(|| {
    ///     ("poneyland", "hoho".to_string())
    /// });
    ///
    /// assert_eq!(map["poneyland"], "hoho".to_string());
    /// ```
    #[inline]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn or_insert_with<F>(self, default: F) -> (&'a mut K, &'a mut V)
    where
        F: FnOnce() -> (K, V),
        K: Hash,
        S: BuildHasher,
    {
        match self {
            RawEntryMut::Occupied(entry) => entry.into_key_value(),
            RawEntryMut::Vacant(entry) => {
                let (k, v) = default();
                entry.insert(k, v)
            }
        }
    }

    /// 在任何潜在的插入 map 之前，提供对占用条目的就地可变访问。
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(hash_raw_entry)]
    /// use std::collections::HashMap;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    ///
    /// map.raw_entry_mut()
    ///    .from_key("poneyland")
    ///    .and_modify(|_k, v| { *v += 1 })
    ///    .or_insert("poneyland", 42);
    /// assert_eq!(map["poneyland"], 42);
    ///
    /// map.raw_entry_mut()
    ///    .from_key("poneyland")
    ///    .and_modify(|_k, v| { *v += 1 })
    ///    .or_insert("poneyland", 0);
    /// assert_eq!(map["poneyland"], 43);
    /// ```
    #[inline]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut K, &mut V),
    {
        match self {
            RawEntryMut::Occupied(mut entry) => {
                {
                    let (k, v) = entry.get_key_value_mut();
                    f(k, v);
                }
                RawEntryMut::Occupied(entry)
            }
            RawEntryMut::Vacant(entry) => RawEntryMut::Vacant(entry),
        }
    }
}

impl<'a, K, V, S> RawOccupiedEntryMut<'a, K, V, S> {
    /// 获取条目中键的引用。
    #[inline]
    #[must_use]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn key(&self) -> &K {
        self.base.key()
    }

    /// 获取条目中键的可变引用。
    #[inline]
    #[must_use]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn key_mut(&mut self) -> &mut K {
        self.base.key_mut()
    }

    /// 将条目转换为变量引用中的键，并将生命周期绑定到 map 本身。
    ///
    #[inline]
    #[must_use = "`self` will be dropped if the result is not used"]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn into_key(self) -> &'a mut K {
        self.base.into_key()
    }

    /// 获取条目中值的引用。
    #[inline]
    #[must_use]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn get(&self) -> &V {
        self.base.get()
    }

    /// 将 `OccupiedEntry` 转换为条目中带有生命周期绑定到 map 本身的值的变量引用。
    ///
    #[inline]
    #[must_use = "`self` will be dropped if the result is not used"]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn into_mut(self) -> &'a mut V {
        self.base.into_mut()
    }

    /// 获取条目中的值的可变引用。
    #[inline]
    #[must_use]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn get_mut(&mut self) -> &mut V {
        self.base.get_mut()
    }

    /// 获取条目中键和值的引用。
    #[inline]
    #[must_use]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn get_key_value(&mut self) -> (&K, &V) {
        self.base.get_key_value()
    }

    /// 获取条目中键和值的可变引用。
    #[inline]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn get_key_value_mut(&mut self) -> (&mut K, &mut V) {
        self.base.get_key_value_mut()
    }

    /// 将 `OccupiedEntry` 转换为条目中的键和值的变量引用，并将生命周期绑定到 map 本身。
    ///
    #[inline]
    #[must_use = "`self` will be dropped if the result is not used"]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn into_key_value(self) -> (&'a mut K, &'a mut V) {
        self.base.into_key_value()
    }

    /// 设置条目的值，并返回条目的旧值。
    #[inline]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn insert(&mut self, value: V) -> V {
        self.base.insert(value)
    }

    /// 设置条目的值，并返回条目的旧值。
    #[inline]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn insert_key(&mut self, key: K) -> K {
        self.base.insert_key(key)
    }

    /// 从条目中取出值，然后将其返回。
    #[inline]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn remove(self) -> V {
        self.base.remove()
    }

    /// 从 map 获取键和值的所有权。
    #[inline]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn remove_entry(self) -> (K, V) {
        self.base.remove_entry()
    }
}

impl<'a, K, V, S> RawVacantEntryMut<'a, K, V, S> {
    /// 用 `VacantEntry` 的键设置条目的值，并返回对它的可变引用。
    ///
    #[inline]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn insert(self, key: K, value: V) -> (&'a mut K, &'a mut V)
    where
        K: Hash,
        S: BuildHasher,
    {
        self.base.insert(key, value)
    }

    /// 使用 VacantEntry 的键设置条目的值，并为其返回变量引用。
    ///
    #[inline]
    #[unstable(feature = "hash_raw_entry", issue = "56167")]
    pub fn insert_hashed_nocheck(self, hash: u64, key: K, value: V) -> (&'a mut K, &'a mut V)
    where
        K: Hash,
        S: BuildHasher,
    {
        self.base.insert_hashed_nocheck(hash, key, value)
    }
}

#[unstable(feature = "hash_raw_entry", issue = "56167")]
impl<K, V, S> Debug for RawEntryBuilderMut<'_, K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawEntryBuilder").finish_non_exhaustive()
    }
}

#[unstable(feature = "hash_raw_entry", issue = "56167")]
impl<K: Debug, V: Debug, S> Debug for RawEntryMut<'_, K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            RawEntryMut::Vacant(ref v) => f.debug_tuple("RawEntry").field(v).finish(),
            RawEntryMut::Occupied(ref o) => f.debug_tuple("RawEntry").field(o).finish(),
        }
    }
}

#[unstable(feature = "hash_raw_entry", issue = "56167")]
impl<K: Debug, V: Debug, S> Debug for RawOccupiedEntryMut<'_, K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawOccupiedEntryMut")
            .field("key", self.key())
            .field("value", self.get())
            .finish_non_exhaustive()
    }
}

#[unstable(feature = "hash_raw_entry", issue = "56167")]
impl<K, V, S> Debug for RawVacantEntryMut<'_, K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawVacantEntryMut").finish_non_exhaustive()
    }
}

#[unstable(feature = "hash_raw_entry", issue = "56167")]
impl<K, V, S> Debug for RawEntryBuilder<'_, K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawEntryBuilder").finish_non_exhaustive()
    }
}

/// map 中单个条目的视图，该条目可能是空的或被已占用。
///
/// `enum` 是根据 [`HashMap`] 上的 [`entry`] 方法构造的。
///
/// [`entry`]: HashMap::entry
#[stable(feature = "rust1", since = "1.0.0")]
#[cfg_attr(not(test), rustc_diagnostic_item = "HashMapEntry")]
pub enum Entry<'a, K: 'a, V: 'a> {
    /// 一个被占用的条目。
    #[stable(feature = "rust1", since = "1.0.0")]
    Occupied(#[stable(feature = "rust1", since = "1.0.0")] OccupiedEntry<'a, K, V>),

    /// 一个空的条目。
    #[stable(feature = "rust1", since = "1.0.0")]
    Vacant(#[stable(feature = "rust1", since = "1.0.0")] VacantEntry<'a, K, V>),
}

#[stable(feature = "debug_hash_map", since = "1.12.0")]
impl<K: Debug, V: Debug> Debug for Entry<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Vacant(ref v) => f.debug_tuple("Entry").field(v).finish(),
            Occupied(ref o) => f.debug_tuple("Entry").field(o).finish(),
        }
    }
}

/// `HashMap` 中已占用条目的视图。
/// 它是 [`Entry`] 枚举的一部分。
#[stable(feature = "rust1", since = "1.0.0")]
pub struct OccupiedEntry<'a, K: 'a, V: 'a> {
    base: base::RustcOccupiedEntry<'a, K, V>,
}

#[stable(feature = "debug_hash_map", since = "1.12.0")]
impl<K: Debug, V: Debug> Debug for OccupiedEntry<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OccupiedEntry")
            .field("key", self.key())
            .field("value", self.get())
            .finish_non_exhaustive()
    }
}

/// `HashMap` 中空闲条目的视图。
/// 它是 [`Entry`] 枚举的一部分。
#[stable(feature = "rust1", since = "1.0.0")]
pub struct VacantEntry<'a, K: 'a, V: 'a> {
    base: base::RustcVacantEntry<'a, K, V>,
}

#[stable(feature = "debug_hash_map", since = "1.12.0")]
impl<K: Debug, V> Debug for VacantEntry<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VacantEntry").field(self.key()).finish()
    }
}

/// 当键已经存在时，由 [`try_insert`](HashMap::try_insert) 返回的错误。
///
/// 包含占用的条目和未插入的值。
#[unstable(feature = "map_try_insert", issue = "82766")]
pub struct OccupiedError<'a, K: 'a, V: 'a> {
    /// map 中已被占用的条目。
    pub entry: OccupiedEntry<'a, K, V>,
    /// 未插入的值，因为该条目已被占用。
    pub value: V,
}

#[unstable(feature = "map_try_insert", issue = "82766")]
impl<K: Debug, V: Debug> Debug for OccupiedError<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OccupiedError")
            .field("key", self.entry.key())
            .field("old_value", self.entry.get())
            .field("new_value", &self.value)
            .finish_non_exhaustive()
    }
}

#[unstable(feature = "map_try_insert", issue = "82766")]
impl<'a, K: Debug, V: Debug> fmt::Display for OccupiedError<'a, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "failed to insert {:?}, key {:?} already exists with value {:?}",
            self.value,
            self.entry.key(),
            self.entry.get(),
        )
    }
}

#[unstable(feature = "map_try_insert", issue = "82766")]
impl<'a, K: fmt::Debug, V: fmt::Debug> Error for OccupiedError<'a, K, V> {
    #[allow(deprecated)]
    fn description(&self) -> &str {
        "key already exists"
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<'a, K, V, S> IntoIterator for &'a HashMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    #[inline]
    #[rustc_lint_query_instability]
    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<'a, K, V, S> IntoIterator for &'a mut HashMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    #[inline]
    #[rustc_lint_query_instability]
    fn into_iter(self) -> IterMut<'a, K, V> {
        self.iter_mut()
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<K, V, S> IntoIterator for HashMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    /// 创建一个消耗迭代器，即一个将任意键值对以任意顺序移出 map 的迭代器。
    /// 调用后不能使用 map。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let map = HashMap::from([
    ///     ("a", 1),
    ///     ("b", 2),
    ///     ("c", 3),
    /// ]);
    ///
    /// // .iter() 无法使用
    /// let vec: Vec<(&str, i32)> = map.into_iter().collect();
    /// ```
    ///
    #[inline]
    #[rustc_lint_query_instability]
    fn into_iter(self) -> IntoIter<K, V> {
        IntoIter { base: self.base.into_iter() }
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    #[inline]
    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        self.base.next()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.base.size_hint()
    }
}
#[stable(feature = "rust1", since = "1.0.0")]
impl<K, V> ExactSizeIterator for Iter<'_, K, V> {
    #[inline]
    fn len(&self) -> usize {
        self.base.len()
    }
}

#[stable(feature = "fused", since = "1.26.0")]
impl<K, V> FusedIterator for Iter<'_, K, V> {}

#[stable(feature = "rust1", since = "1.0.0")]
impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    #[inline]
    fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
        self.base.next()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.base.size_hint()
    }
}
#[stable(feature = "rust1", since = "1.0.0")]
impl<K, V> ExactSizeIterator for IterMut<'_, K, V> {
    #[inline]
    fn len(&self) -> usize {
        self.base.len()
    }
}
#[stable(feature = "fused", since = "1.26.0")]
impl<K, V> FusedIterator for IterMut<'_, K, V> {}

#[stable(feature = "std_debug", since = "1.16.0")]
impl<K, V> fmt::Debug for IterMut<'_, K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    #[inline]
    fn next(&mut self) -> Option<(K, V)> {
        self.base.next()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.base.size_hint()
    }
}
#[stable(feature = "rust1", since = "1.0.0")]
impl<K, V> ExactSizeIterator for IntoIter<K, V> {
    #[inline]
    fn len(&self) -> usize {
        self.base.len()
    }
}
#[stable(feature = "fused", since = "1.26.0")]
impl<K, V> FusedIterator for IntoIter<K, V> {}

#[stable(feature = "std_debug", since = "1.16.0")]
impl<K: Debug, V: Debug> fmt::Debug for IntoIter<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<'a, K, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;

    #[inline]
    fn next(&mut self) -> Option<&'a K> {
        self.inner.next().map(|(k, _)| k)
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
#[stable(feature = "rust1", since = "1.0.0")]
impl<K, V> ExactSizeIterator for Keys<'_, K, V> {
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}
#[stable(feature = "fused", since = "1.26.0")]
impl<K, V> FusedIterator for Keys<'_, K, V> {}

#[stable(feature = "rust1", since = "1.0.0")]
impl<'a, K, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;

    #[inline]
    fn next(&mut self) -> Option<&'a V> {
        self.inner.next().map(|(_, v)| v)
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
#[stable(feature = "rust1", since = "1.0.0")]
impl<K, V> ExactSizeIterator for Values<'_, K, V> {
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}
#[stable(feature = "fused", since = "1.26.0")]
impl<K, V> FusedIterator for Values<'_, K, V> {}

#[stable(feature = "map_values_mut", since = "1.10.0")]
impl<'a, K, V> Iterator for ValuesMut<'a, K, V> {
    type Item = &'a mut V;

    #[inline]
    fn next(&mut self) -> Option<&'a mut V> {
        self.inner.next().map(|(_, v)| v)
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
#[stable(feature = "map_values_mut", since = "1.10.0")]
impl<K, V> ExactSizeIterator for ValuesMut<'_, K, V> {
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}
#[stable(feature = "fused", since = "1.26.0")]
impl<K, V> FusedIterator for ValuesMut<'_, K, V> {}

#[stable(feature = "std_debug", since = "1.16.0")]
impl<K, V: fmt::Debug> fmt::Debug for ValuesMut<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.inner.iter().map(|(_, val)| val)).finish()
    }
}

#[stable(feature = "map_into_keys_values", since = "1.54.0")]
impl<K, V> Iterator for IntoKeys<K, V> {
    type Item = K;

    #[inline]
    fn next(&mut self) -> Option<K> {
        self.inner.next().map(|(k, _)| k)
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
#[stable(feature = "map_into_keys_values", since = "1.54.0")]
impl<K, V> ExactSizeIterator for IntoKeys<K, V> {
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}
#[stable(feature = "map_into_keys_values", since = "1.54.0")]
impl<K, V> FusedIterator for IntoKeys<K, V> {}

#[stable(feature = "map_into_keys_values", since = "1.54.0")]
impl<K: Debug, V> fmt::Debug for IntoKeys<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.inner.iter().map(|(k, _)| k)).finish()
    }
}

#[stable(feature = "map_into_keys_values", since = "1.54.0")]
impl<K, V> Iterator for IntoValues<K, V> {
    type Item = V;

    #[inline]
    fn next(&mut self) -> Option<V> {
        self.inner.next().map(|(_, v)| v)
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
#[stable(feature = "map_into_keys_values", since = "1.54.0")]
impl<K, V> ExactSizeIterator for IntoValues<K, V> {
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}
#[stable(feature = "map_into_keys_values", since = "1.54.0")]
impl<K, V> FusedIterator for IntoValues<K, V> {}

#[stable(feature = "map_into_keys_values", since = "1.54.0")]
impl<K, V: Debug> fmt::Debug for IntoValues<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.inner.iter().map(|(_, v)| v)).finish()
    }
}

#[stable(feature = "drain", since = "1.6.0")]
impl<'a, K, V> Iterator for Drain<'a, K, V> {
    type Item = (K, V);

    #[inline]
    fn next(&mut self) -> Option<(K, V)> {
        self.base.next()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.base.size_hint()
    }
}
#[stable(feature = "drain", since = "1.6.0")]
impl<K, V> ExactSizeIterator for Drain<'_, K, V> {
    #[inline]
    fn len(&self) -> usize {
        self.base.len()
    }
}
#[stable(feature = "fused", since = "1.26.0")]
impl<K, V> FusedIterator for Drain<'_, K, V> {}

#[stable(feature = "std_debug", since = "1.16.0")]
impl<K, V> fmt::Debug for Drain<'_, K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

#[unstable(feature = "hash_drain_filter", issue = "59618")]
impl<K, V, F> Iterator for DrainFilter<'_, K, V, F>
where
    F: FnMut(&K, &mut V) -> bool,
{
    type Item = (K, V);

    #[inline]
    fn next(&mut self) -> Option<(K, V)> {
        self.base.next()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.base.size_hint()
    }
}

#[unstable(feature = "hash_drain_filter", issue = "59618")]
impl<K, V, F> FusedIterator for DrainFilter<'_, K, V, F> where F: FnMut(&K, &mut V) -> bool {}

#[unstable(feature = "hash_drain_filter", issue = "59618")]
impl<'a, K, V, F> fmt::Debug for DrainFilter<'a, K, V, F>
where
    F: FnMut(&K, &mut V) -> bool,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DrainFilter").finish_non_exhaustive()
    }
}

impl<'a, K, V> Entry<'a, K, V> {
    /// 如果为空，则通过插入默认值来确保该值在条目中，并返回对条目中值的可变引用。
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    ///
    /// map.entry("poneyland").or_insert(3);
    /// assert_eq!(map["poneyland"], 3);
    ///
    /// *map.entry("poneyland").or_insert(10) *= 2;
    /// assert_eq!(map["poneyland"], 6);
    /// ```
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(default),
        }
    }

    /// 如果为空，则通过插入默认函数的结果来确保该值在条目中，并返回对条目中值的可变引用。
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut map: HashMap<&str, String> = HashMap::new();
    /// let s = "hoho".to_string();
    ///
    /// map.entry("poneyland").or_insert_with(|| s);
    ///
    /// assert_eq!(map["poneyland"], "hoho".to_string());
    /// ```
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V {
        match self {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(default()),
        }
    }

    /// 如果为空，则通过插入默认函数的结果，确保值在条目中。
    /// 通过为 `.entry(key)` 方法调用期间移动的键提供默认函数引用，此方法可以生成用于插入的键派生值。
    ///
    ///
    /// 提供了对已移动键的引用，因此不需要克隆或复制键，这与 `.or_insert_with(|| ... )` 不同。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut map: HashMap<&str, usize> = HashMap::new();
    ///
    /// map.entry("poneyland").or_insert_with_key(|key| key.chars().count());
    ///
    /// assert_eq!(map["poneyland"], 9);
    /// ```
    ///
    #[inline]
    #[stable(feature = "or_insert_with_key", since = "1.50.0")]
    pub fn or_insert_with_key<F: FnOnce(&K) -> V>(self, default: F) -> &'a mut V {
        match self {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => {
                let value = default(entry.key());
                entry.insert(value)
            }
        }
    }

    /// 返回此条目的键的引用。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    /// assert_eq!(map.entry("poneyland").key(), &"poneyland");
    /// ```
    #[inline]
    #[stable(feature = "map_entry_keys", since = "1.10.0")]
    pub fn key(&self) -> &K {
        match *self {
            Occupied(ref entry) => entry.key(),
            Vacant(ref entry) => entry.key(),
        }
    }

    /// 在任何潜在的插入 map 之前，提供对占用条目的就地可变访问。
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    ///
    /// map.entry("poneyland")
    ///    .and_modify(|e| { *e += 1 })
    ///    .or_insert(42);
    /// assert_eq!(map["poneyland"], 42);
    ///
    /// map.entry("poneyland")
    ///    .and_modify(|e| { *e += 1 })
    ///    .or_insert(42);
    /// assert_eq!(map["poneyland"], 43);
    /// ```
    #[inline]
    #[stable(feature = "entry_and_modify", since = "1.26.0")]
    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut V),
    {
        match self {
            Occupied(mut entry) => {
                f(entry.get_mut());
                Occupied(entry)
            }
            Vacant(entry) => Vacant(entry),
        }
    }

    /// 设置条目的值，并返回 `OccupiedEntry`。
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(entry_insert)]
    /// use std::collections::HashMap;
    ///
    /// let mut map: HashMap<&str, String> = HashMap::new();
    /// let entry = map.entry("poneyland").insert_entry("hoho".to_string());
    ///
    /// assert_eq!(entry.key(), &"poneyland");
    /// ```
    #[inline]
    #[unstable(feature = "entry_insert", issue = "65225")]
    pub fn insert_entry(self, value: V) -> OccupiedEntry<'a, K, V> {
        match self {
            Occupied(mut entry) => {
                entry.insert(value);
                entry
            }
            Vacant(entry) => entry.insert_entry(value),
        }
    }
}

impl<'a, K, V: Default> Entry<'a, K, V> {
    /// 如果为空，则通过插入默认值来确保值在条目中，并向条目中的值返回变量引用。
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() {
    /// use std::collections::HashMap;
    ///
    /// let mut map: HashMap<&str, Option<u32>> = HashMap::new();
    /// map.entry("poneyland").or_default();
    ///
    /// assert_eq!(map["poneyland"], None);
    /// # }
    /// ```
    #[inline]
    #[stable(feature = "entry_or_default", since = "1.28.0")]
    pub fn or_default(self) -> &'a mut V {
        match self {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(Default::default()),
        }
    }
}

impl<'a, K, V> OccupiedEntry<'a, K, V> {
    /// 获取条目中键的引用。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    /// map.entry("poneyland").or_insert(12);
    /// assert_eq!(map.entry("poneyland").key(), &"poneyland");
    /// ```
    #[inline]
    #[stable(feature = "map_entry_keys", since = "1.10.0")]
    pub fn key(&self) -> &K {
        self.base.key()
    }

    /// 从 map 获取键和值的所有权。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use std::collections::hash_map::Entry;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    /// map.entry("poneyland").or_insert(12);
    ///
    /// if let Entry::Occupied(o) = map.entry("poneyland") {
    ///     // 我们从 map 中删除了这个条目。
    ///     o.remove_entry();
    /// }
    ///
    /// assert_eq!(map.contains_key("poneyland"), false);
    /// ```
    #[inline]
    #[stable(feature = "map_entry_recover_keys2", since = "1.12.0")]
    pub fn remove_entry(self) -> (K, V) {
        self.base.remove_entry()
    }

    /// 获取条目中值的引用。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use std::collections::hash_map::Entry;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    /// map.entry("poneyland").or_insert(12);
    ///
    /// if let Entry::Occupied(o) = map.entry("poneyland") {
    ///     assert_eq!(o.get(), &12);
    /// }
    /// ```
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn get(&self) -> &V {
        self.base.get()
    }

    /// 获取条目中的值的可变引用。
    ///
    /// 如果需要对 `OccupiedEntry` 的引用，而这可能会使 `Entry` 值的破坏失效，请参见 [`into_mut`]。
    ///
    ///
    /// [`into_mut`]: Self::into_mut
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use std::collections::hash_map::Entry;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    /// map.entry("poneyland").or_insert(12);
    ///
    /// assert_eq!(map["poneyland"], 12);
    /// if let Entry::Occupied(mut o) = map.entry("poneyland") {
    ///     *o.get_mut() += 10;
    ///     assert_eq!(*o.get(), 22);
    ///
    ///     // 我们可以多次使用同一个 Entry。
    ///     *o.get_mut() += 2;
    /// }
    ///
    /// assert_eq!(map["poneyland"], 24);
    /// ```
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn get_mut(&mut self) -> &mut V {
        self.base.get_mut()
    }

    /// 将 `OccupiedEntry` 转换为条目中带有生命周期绑定到 map 本身的值的变量引用。
    ///
    ///
    /// 如果需要多次引用 `OccupiedEntry`，请参见 [`get_mut`]。
    ///
    /// [`get_mut`]: Self::get_mut
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use std::collections::hash_map::Entry;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    /// map.entry("poneyland").or_insert(12);
    ///
    /// assert_eq!(map["poneyland"], 12);
    /// if let Entry::Occupied(o) = map.entry("poneyland") {
    ///     *o.into_mut() += 10;
    /// }
    ///
    /// assert_eq!(map["poneyland"], 22);
    /// ```
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn into_mut(self) -> &'a mut V {
        self.base.into_mut()
    }

    /// 设置条目的值，并返回条目的旧值。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use std::collections::hash_map::Entry;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    /// map.entry("poneyland").or_insert(12);
    ///
    /// if let Entry::Occupied(mut o) = map.entry("poneyland") {
    ///     assert_eq!(o.insert(15), 12);
    /// }
    ///
    /// assert_eq!(map["poneyland"], 15);
    /// ```
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn insert(&mut self, value: V) -> V {
        self.base.insert(value)
    }

    /// 从条目中取出值，然后将其返回。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use std::collections::hash_map::Entry;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    /// map.entry("poneyland").or_insert(12);
    ///
    /// if let Entry::Occupied(o) = map.entry("poneyland") {
    ///     assert_eq!(o.remove(), 12);
    /// }
    ///
    /// assert_eq!(map.contains_key("poneyland"), false);
    /// ```
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn remove(self) -> V {
        self.base.remove()
    }

    /// 替换条目，返回旧的键和值。
    /// 哈希 map 中的新键将是用于创建此条目的键。
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(map_entry_replace)]
    /// use std::collections::hash_map::{Entry, HashMap};
    /// use std::rc::Rc;
    ///
    /// let mut map: HashMap<Rc<String>, u32> = HashMap::new();
    /// map.insert(Rc::new("Stringthing".to_string()), 15);
    ///
    /// let my_key = Rc::new("Stringthing".to_string());
    ///
    /// if let Entry::Occupied(entry) = map.entry(my_key) {
    ///     // 同时用我们其他键的句柄代替键。
    ///     let (old_key, old_value): (Rc<String>, u32) = entry.replace_entry(16);
    /// }
    ///
    /// ```
    #[inline]
    #[unstable(feature = "map_entry_replace", issue = "44286")]
    pub fn replace_entry(self, value: V) -> (K, V) {
        self.base.replace_entry(value)
    }

    /// 用用于创建此条目的键替换哈希 map 中的键。
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(map_entry_replace)]
    /// use std::collections::hash_map::{Entry, HashMap};
    /// use std::rc::Rc;
    ///
    /// let mut map: HashMap<Rc<String>, u32> = HashMap::new();
    /// let known_strings: Vec<Rc<String>> = Vec::new();
    ///
    /// // 初始化已知的字符串，运行程序等
    ///
    /// reclaim_memory(&mut map, &known_strings);
    ///
    /// fn reclaim_memory(map: &mut HashMap<Rc<String>, u32>, known_strings: &[Rc<String>] ) {
    ///     for s in known_strings {
    ///         if let Entry::Occupied(entry) = map.entry(Rc::clone(s)) {
    ///             // 将条目的键替换为我们在 `known_strings` 中的版本。
    ///             entry.replace_key();
    ///         }
    ///     }
    /// }
    /// ```
    #[inline]
    #[unstable(feature = "map_entry_replace", issue = "44286")]
    pub fn replace_key(self) -> K {
        self.base.replace_key()
    }
}

impl<'a, K: 'a, V: 'a> VacantEntry<'a, K, V> {
    /// 获取对通过 `VacantEntry` 插入值时将使用的键的引用。
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    /// assert_eq!(map.entry("poneyland").key(), &"poneyland");
    /// ```
    #[inline]
    #[stable(feature = "map_entry_keys", since = "1.10.0")]
    pub fn key(&self) -> &K {
        self.base.key()
    }

    /// 取得键的所有权。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use std::collections::hash_map::Entry;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    ///
    /// if let Entry::Vacant(v) = map.entry("poneyland") {
    ///     v.into_key();
    /// }
    /// ```
    #[inline]
    #[stable(feature = "map_entry_recover_keys2", since = "1.12.0")]
    pub fn into_key(self) -> K {
        self.base.into_key()
    }

    /// 用 `VacantEntry` 的键设置条目的值，并返回对它的可变引用。
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use std::collections::hash_map::Entry;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    ///
    /// if let Entry::Vacant(o) = map.entry("poneyland") {
    ///     o.insert(37);
    /// }
    /// assert_eq!(map["poneyland"], 37);
    /// ```
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn insert(self, value: V) -> &'a mut V {
        self.base.insert(value)
    }

    /// 使用 `VacantEntry` 的键设置条目的值，并返回 `OccupiedEntry`。
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(entry_insert)]
    /// use std::collections::HashMap;
    /// use std::collections::hash_map::Entry;
    ///
    /// let mut map: HashMap<&str, u32> = HashMap::new();
    ///
    /// if let Entry::Vacant(o) = map.entry("poneyland") {
    ///     o.insert_entry(37);
    /// }
    /// assert_eq!(map["poneyland"], 37);
    /// ```
    #[inline]
    #[unstable(feature = "entry_insert", issue = "65225")]
    pub fn insert_entry(self, value: V) -> OccupiedEntry<'a, K, V> {
        let base = self.base.insert_entry(value);
        OccupiedEntry { base }
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<K, V, S> FromIterator<(K, V)> for HashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher + Default,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> HashMap<K, V, S> {
        let mut map = HashMap::with_hasher(Default::default());
        map.extend(iter);
        map
    }
}

/// 插入迭代器中的所有新键值，并用迭代器返回的新值替换现有键中的值。
///
#[stable(feature = "rust1", since = "1.0.0")]
impl<K, V, S> Extend<(K, V)> for HashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    #[inline]
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        self.base.extend(iter)
    }

    #[inline]
    fn extend_one(&mut self, (k, v): (K, V)) {
        self.base.insert(k, v);
    }

    #[inline]
    fn extend_reserve(&mut self, additional: usize) {
        self.base.extend_reserve(additional);
    }
}

#[stable(feature = "hash_extend_copy", since = "1.4.0")]
impl<'a, K, V, S> Extend<(&'a K, &'a V)> for HashMap<K, V, S>
where
    K: Eq + Hash + Copy,
    V: Copy,
    S: BuildHasher,
{
    #[inline]
    fn extend<T: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: T) {
        self.base.extend(iter)
    }

    #[inline]
    fn extend_one(&mut self, (&k, &v): (&'a K, &'a V)) {
        self.base.insert(k, v);
    }

    #[inline]
    fn extend_reserve(&mut self, additional: usize) {
        Extend::<(K, V)>::extend_reserve(self, additional)
    }
}

/// `RandomState` 是 [`HashMap`] 类型的默认状态。
///
/// 特定的实例 `RandomState` 将创建 [`Hasher`] 的相同实例，但是由两个不同的 `RandomState` 实例创建的哈希对于相同的值不太可能产生相同的结果。
///
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use std::collections::hash_map::RandomState;
///
/// let s = RandomState::new();
/// let mut map = HashMap::with_hasher(s);
/// map.insert(1, 2);
/// ```
///
#[derive(Clone)]
#[stable(feature = "hashmap_build_hasher", since = "1.7.0")]
pub struct RandomState {
    k0: u64,
    k1: u64,
}

impl RandomState {
    /// 创建一个用随机键初始化的新 `RandomState`。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::hash_map::RandomState;
    ///
    /// let s = RandomState::new();
    /// ```
    #[inline]
    #[allow(deprecated)]
    // rand
    #[must_use]
    #[stable(feature = "hashmap_build_hasher", since = "1.7.0")]
    pub fn new() -> RandomState {
        // 从历史上看，此函数不缓存操作系统中的键，而总是简单地两次调用 `rand::thread_rng().gen()`。
        // 但是，在 #31356 中发现，由于我们定期从操作系统重新 seed，所以在线程上创建许多 hashmap 时，这可能会导致速度过慢。
        //
        // 为了解决此性能陷阱，我们按线程缓存了第一组随机生成的密钥。
        //
        // 后来在 #36481 中，发现确定性迭代顺序可以允许某种形式的 DOS 攻击。
        // 为了解决这个问题，我们在每次 RandomState 创建时增加一个种子，为每个对应的 HashMap 赋予不同的迭代顺序。
        //
        //
        //
        //
        thread_local!(static KEYS: Cell<(u64, u64)> = {
            Cell::new(sys::hashmap_random_keys())
        });

        KEYS.with(|keys| {
            let (k0, k1) = keys.get();
            keys.set((k0.wrapping_add(1), k1));
            RandomState { k0, k1 }
        })
    }
}

#[stable(feature = "hashmap_build_hasher", since = "1.7.0")]
impl BuildHasher for RandomState {
    type Hasher = DefaultHasher;
    #[inline]
    #[allow(deprecated)]
    fn build_hasher(&self) -> DefaultHasher {
        DefaultHasher(SipHasher13::new_with_keys(self.k0, self.k1))
    }
}

/// [`RandomState`] 使用的默认 [`Hasher`]。
///
/// 未指定内部算法，因此不应在发布时依赖它及其哈希值。
///
#[stable(feature = "hashmap_default_hasher", since = "1.13.0")]
#[allow(deprecated)]
#[derive(Clone, Debug)]
pub struct DefaultHasher(SipHasher13);

impl DefaultHasher {
    /// 创建一个新的 `DefaultHasher`。
    ///
    /// 不保证此哈希值与所有其他 `DefaultHasher` 实例相同，但与通过 `new` 或 `default` 创建的所有其他 `DefaultHasher` 实例相同。
    ///
    ///
    #[stable(feature = "hashmap_default_hasher", since = "1.13.0")]
    #[inline]
    #[allow(deprecated)]
    #[rustc_const_unstable(feature = "const_hash", issue = "104061")]
    #[must_use]
    pub const fn new() -> DefaultHasher {
        DefaultHasher(SipHasher13::new_with_keys(0, 0))
    }
}

#[stable(feature = "hashmap_default_hasher", since = "1.13.0")]
impl Default for DefaultHasher {
    /// 使用 [`new`] 创建一个新的 `DefaultHasher`。
    /// 有关更多信息，请参见其文档。
    ///
    /// [`new`]: DefaultHasher::new
    #[inline]
    fn default() -> DefaultHasher {
        DefaultHasher::new()
    }
}

#[stable(feature = "hashmap_default_hasher", since = "1.13.0")]
impl Hasher for DefaultHasher {
    // 底层 `SipHasher13` 不会覆盖其他 `write_*` 方法，所以这里不转发也没关系。
    //

    #[inline]
    fn write(&mut self, msg: &[u8]) {
        self.0.write(msg)
    }

    #[inline]
    fn write_str(&mut self, s: &str) {
        self.0.write_str(s);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0.finish()
    }
}

#[stable(feature = "hashmap_build_hasher", since = "1.7.0")]
impl Default for RandomState {
    /// 创建一个新的 `RandomState`。
    #[inline]
    fn default() -> RandomState {
        RandomState::new()
    }
}

#[stable(feature = "std_debug", since = "1.16.0")]
impl fmt::Debug for RandomState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RandomState").finish_non_exhaustive()
    }
}

#[inline]
fn map_entry<'a, K: 'a, V: 'a>(raw: base::RustcEntry<'a, K, V>) -> Entry<'a, K, V> {
    match raw {
        base::RustcEntry::Occupied(base) => Entry::Occupied(OccupiedEntry { base }),
        base::RustcEntry::Vacant(base) => Entry::Vacant(VacantEntry { base }),
    }
}

#[inline]
pub(super) fn map_try_reserve_error(err: hashbrown::TryReserveError) -> TryReserveError {
    match err {
        hashbrown::TryReserveError::CapacityOverflow => {
            TryReserveErrorKind::CapacityOverflow.into()
        }
        hashbrown::TryReserveError::AllocError { layout } => {
            TryReserveErrorKind::AllocError { layout, non_exhaustive: () }.into()
        }
    }
}

#[inline]
fn map_raw_entry<'a, K: 'a, V: 'a, S: 'a>(
    raw: base::RawEntryMut<'a, K, V, S>,
) -> RawEntryMut<'a, K, V, S> {
    match raw {
        base::RawEntryMut::Occupied(base) => RawEntryMut::Occupied(RawOccupiedEntryMut { base }),
        base::RawEntryMut::Vacant(base) => RawEntryMut::Vacant(RawVacantEntryMut { base }),
    }
}

#[allow(dead_code)]
fn assert_covariance() {
    fn map_key<'new>(v: HashMap<&'static str, u8>) -> HashMap<&'new str, u8> {
        v
    }
    fn map_val<'new>(v: HashMap<u8, &'static str>) -> HashMap<u8, &'new str> {
        v
    }
    fn iter_key<'a, 'new>(v: Iter<'a, &'static str, u8>) -> Iter<'a, &'new str, u8> {
        v
    }
    fn iter_val<'a, 'new>(v: Iter<'a, u8, &'static str>) -> Iter<'a, u8, &'new str> {
        v
    }
    fn into_iter_key<'new>(v: IntoIter<&'static str, u8>) -> IntoIter<&'new str, u8> {
        v
    }
    fn into_iter_val<'new>(v: IntoIter<u8, &'static str>) -> IntoIter<u8, &'new str> {
        v
    }
    fn keys_key<'a, 'new>(v: Keys<'a, &'static str, u8>) -> Keys<'a, &'new str, u8> {
        v
    }
    fn keys_val<'a, 'new>(v: Keys<'a, u8, &'static str>) -> Keys<'a, u8, &'new str> {
        v
    }
    fn values_key<'a, 'new>(v: Values<'a, &'static str, u8>) -> Values<'a, &'new str, u8> {
        v
    }
    fn values_val<'a, 'new>(v: Values<'a, u8, &'static str>) -> Values<'a, u8, &'new str> {
        v
    }
    fn drain<'new>(
        d: Drain<'static, &'static str, &'static str>,
    ) -> Drain<'new, &'new str, &'new str> {
        d
    }
}