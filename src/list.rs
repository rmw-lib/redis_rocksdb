use crate::{Bytes, Direction};
use crate::Error;

trait List {
    fn blpop<K: Bytes, V: Bytes>(key: K, timeout: i64) -> Result<V, Error> {
        unimplemented!()
    }
    fn brpop<K: Bytes, V: Bytes>(key: K, timeout: i64) -> Result<V, Error> {
        unimplemented!()
    }
    fn brpoplpush<K: Bytes, V: Bytes>(srckey: K, dstkey: K, timeout: i64) -> Result<V, Error> {
        unimplemented!()
    }
    fn lindex<K: Bytes, V: Bytes>(key: K, index: i32) -> Result<V, Error>;
    fn linsert_before<K: Bytes, P: Bytes, V: Bytes>(key: K, pivot: P, value: V) -> Result<(), Error>;
    fn linsert_after<K: Bytes, P: Bytes, V: Bytes>(key: K, pivot: P, value: V) -> Result<(), Error>;

    fn llen<K: Bytes>(key: K) -> Result<i32, Error>;

    fn lmove<K: Bytes, V: Bytes>(srckey: K, dstkey: K, src_dir: Direction, dst_dir: Direction) -> Result<V, Error> {
        unimplemented!()
    }
    fn lmpop<K: Bytes>(numkeys: i32, key: K, dir: Direction, count: i32);
    fn lpop<K: Bytes, V: Bytes>(key: K) -> Result<V, Error>;
    fn lpush<K: Bytes, V: Bytes>(key: K, value: V) -> Result<(), Error>;

    fn lpush_exists<K: Bytes, V: Bytes>(key: K, value: V) -> Result<(), Error>;

    fn lrange<K: Bytes, V: Bytes>(key: K, start: i32, stop: i32) -> Result<Vec<V>, Error>;
    /// COUNT 的值可以是以下几种：
    /// count > 0 : 从表头开始向表尾搜索，移除与 VALUE 相等的元素，数量为 COUNT。
    /// count < 0 : 从表尾开始向表头搜索，移除与 VALUE 相等的元素，数量为 COUNT 的绝对值。
    /// count = 0 : 移除表中所有与 VALUE 相等的值
    fn lrem<K: Bytes, V: Bytes>(key: K, count: i32, value: V) -> Result<V, Error>;
    /// 保留指定区间内的元素，不在指定区间之内的元素都将被删除, 反回删除的元素数量
    fn ltrim<K: Bytes>(key: K, start: i32, stop: i32) -> Result<i32, Error>;

    fn lset<K: Bytes, V: Bytes>(key: K, index: i32, value: V);
    /// 移除列表的最后一个元素，返回值为移除的元素
    fn rpop<K: Bytes>(key: K, count: Option<i32>);
    /// 移除列表的最后一个元素，并将该元素添加到另一个列表并返回
    fn rpoplpush<K: Bytes, V: Bytes>(key: K, dstkey: K) -> Result<V, Error>;
    /// 添加到列表尾部
    fn rpush<K: Bytes, V: Bytes>(key: K, value: V) -> Result<(), Error>;
    /// 为已经存在的列表添加值， 添加到尾部
    fn rpush_exists<K: Bytes, V: Bytes>(key: K, value: V) -> Result<(), Error>;
}