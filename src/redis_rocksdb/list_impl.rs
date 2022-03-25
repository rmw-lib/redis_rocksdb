use ckb_rocksdb::prelude::{Delete, Put, TransactionBegin};

use crate::{Bytes, Direction, Error, LenType, RedisList, RedisRocksdb};
use crate::redis_rocksdb::quick_list::QuickList;
use crate::redis_rocksdb::quick_list_node::QuickListNode;
use crate::redis_rocksdb::zip_list::ZipList;

/// [see] (https://xindoo.blog.csdn.net/article/details/109150975)
/// ssdb没有实现list，只实现了queue
///
/// redis中的list使用quicklist与ziplist实现
impl RedisList for RedisRocksdb {
    fn blpop<K: Bytes, V: Bytes>(&mut self, key: K, timeout: i64) -> Result<V, Error> {
        todo!()
    }

    fn brpop<K: Bytes, V: Bytes>(&mut self, key: K, timeout: i64) -> Result<V, Error> {
        todo!()
    }

    fn brpoplpush<K: Bytes, V: Bytes>(&mut self, srckey: K, dstkey: K, timeout: i64) -> Result<V, Error> {
        todo!()
    }

    fn lindex<K: Bytes>(&self, key: K, index: i32) -> Result<Vec<u8>, Error> {
        let t = QuickList::get(&self.db, key.as_ref())?.ok_or(Error::not_find("key of list"))?;
        if index >= t.len_list() as i32 {
            return Err(Error::not_find(&format!("the index {}", index)));
        }
        //todo read only
        let tr = self.db.transaction_default();
        let node_key = t.left().ok_or(Error::none_error("left of quick list"))?;
        let mut node = QuickListNode::get(&tr, node_key.as_ref())?.ok_or(Error::none_error("left node"))?;
        let mut it_index = 0i32;
        it_index += node.len_list() as i32;
        while index >= it_index {
            let next_key = node.right().ok_or(Error::none_error("right node"))?;
            node = QuickListNode::get(&tr, next_key.as_ref())?.ok_or(Error::none_error("next node"))?;
            it_index += node.len_list() as i32;
        }

        let value_key = node.values_key().ok_or(Error::none_error("value key"))?;
        let zip = ZipList::get(&tr, value_key.as_ref())?.ok_or(Error::none_error("zip list"))?;
        let zip_index = index - (it_index - node.len_list() as i32);
        let v = zip.index(zip_index).ok_or(Error::not_find(&format!("the index {}", index)))?;
        tr.commit()?;
        Ok(v.to_vec())
    }
    fn linsert_before<K: Bytes, P: Bytes, V: Bytes>(&mut self, key: K, pivot: P, value: V) -> Result<i32, Error> {
        let mut quick = {
            match QuickList::get(&self.db, key.as_ref())? {
                None => return Ok(0),
                Some(q) => q
            }
        };
        let tr = self.db.transaction_default();
        let result = quick.list_insert(&tr, key.as_ref(), pivot.as_ref(), value.as_ref(), ZipList::insert_value_left)?;
        tr.commit()?;
        Ok(result)
    }

    fn linsert_after<K: Bytes, P: Bytes, V: Bytes>(&mut self, key: K, pivot: P, value: V) -> Result<i32, Error> {
        let mut quick = {
            match QuickList::get(&self.db, key.as_ref())? {
                None => return Ok(0),
                Some(q) => q
            }
        };

        let tr = self.db.transaction_default();
        let result = quick.list_insert(&tr, key.as_ref(), pivot.as_ref(), value.as_ref(), ZipList::insert_value_right)?;
        tr.commit()?;
        Ok(result)
    }

    fn llen<K: Bytes>(&self, key: K) -> Result<i32, Error> {
        match QuickList::get(&self.db, key.as_ref())? {
            None => Ok(-1),
            Some(quick) => {
                Ok(quick.len_list() as i32)
            }
        }
    }

    fn lpop<K: Bytes>(&mut self, list_key: K) -> Result<Vec<u8>, Error> {
        let tr = self.db.transaction_default();
        let mut quick = match QuickList::get(&self.db, list_key.as_ref())? {
            None => return Err(Error::not_find("key of list")),
            Some(q) => q
        };
        let node_key = quick.left().ok_or(Error::none_error("left key"))?.clone();
        let mut node = QuickListNode::get(&tr, node_key.as_ref())?.ok_or(Error::none_error("left node"))?;
        let zip_key = node.values_key().ok_or(Error::none_error("zip key"))?.clone();
        let mut zip = ZipList::get(&tr, zip_key.as_ref())?.ok_or(Error::none_error("zip list"))?;
        let value = zip.pop_left();

        if zip.len() == 0 {
            //没有数据，删除quick list node
            if quick.len_node() == 1 {
                tr.delete(zip_key)?;
                tr.delete(node_key)?;
                quick.set_right(&None);
                quick.set_left(&None);
                quick.set_len_list(0);
                quick.set_len_node(0);
                tr.put(list_key.as_ref(), quick)?;
            } else {
                let left = node.right();
                quick.set_right(&node.left());
                quick.set_len_node(quick.len_node() - 1);
                quick.set_len_list(quick.len_list() - 1);
                tr.delete(zip_key)?;
                tr.delete(node_key)?;
                tr.put(list_key.as_ref(), quick)?;
            }
        } else {
            node.set_len_list(zip.len());
            node.set_len_bytes(zip.as_ref().len() as u32);
            quick.set_len_list(quick.len_list() - 1);

            tr.put(zip_key, zip)?;
            tr.put(node_key, node.as_ref())?;
        }

        tr.commit()?;
        Ok(value)
    }

    fn lpush<K: Bytes, V: Bytes>(&mut self, list_key: K, value: V) -> Result<i32, Error> {
        let tr = self.db.transaction_default();
        let mut quick = match QuickList::get(&self.db, list_key.as_ref())? {
            None => {
                let mut q = QuickList::new();
                q.init_meta_key(list_key.as_ref());
                q
            }
            Some(q) => q
        };
        let re = quick.lpush(&tr, list_key.as_ref(), value.as_ref())?;
        tr.commit()?;
        Ok(re)
    }

    fn lpush_exists<K: Bytes, V: Bytes>(&mut self, list_key: K, value: V) -> Result<i32, Error> {
        let mut quick = match QuickList::get(&self.db, list_key.as_ref())? {
            None => return Ok(0),
            Some(q) => q
        };
        let tr = self.db.transaction_default();
        let re = quick.lpush(&tr, list_key.as_ref(), value.as_ref())?;
        tr.commit()?;
        Ok(re)
    }


    fn lrange<K: Bytes>(&self, list_key: K, start: i32, stop: i32) -> Result<Vec<Vec<u8>>, Error> {
        let mut result = Vec::new();
        let mut quick = match QuickList::get(&self.db, list_key.as_ref())? {
            None => return Ok(result),
            Some(q) => q
        };
        if quick.len_list() < 1 {
            return Ok(result);
        }

        let start_index = ZipList::count_index(quick.len_list() as i32, start) as usize;
        let stop_index = ZipList::count_index(quick.len_list() as i32, stop) as usize;
        if start_index > stop_index {
            return Ok(result)
        }

        //todo read only
        let tr = self.db.transaction_default();

        let mut node_key = quick.left().ok_or(Error::none_error("left key"))?;
        let mut node = QuickListNode::get(&tr, node_key.as_ref())?.ok_or(Error::none_error("quick list node"))?;
        let mut offset = 0usize;
        loop {

            let len_zip = node.len_list();
            if start_index < len_zip as usize + offset {
                let temp = ZipList::count_in_index(len_zip, offset, start_index, stop_index);
                if let Some((start_in, stop_in)) = temp {
                    let zip_key = node.values_key().ok_or(Error::none_error("zip key"))?;
                    let zip = ZipList::get(&tr, zip_key.as_ref())?.ok_or(Error::none_error("zip"))?;
                    let one = zip.range(start_in as i32, stop_in as i32);
                    result.extend(one);
                }//else 是没有数据
            }

            if stop_index < len_zip as usize + offset{
                //取了所有数据
                break
            }

            if let Some(t) = node.right() {
                node = QuickListNode::get(&tr, t.as_ref())?.ok_or(Error::none_error("quick list node"))?;
            }else{
                // 没有更多的节点
                break;
            }
        }

        tr.commit()?;
        Ok(result)
    }

    fn lrem<K: Bytes, V: Bytes>(&mut self, list_key: K, count: i32, value: V) -> Result<LenType, Error> {
        let mut quick = match QuickList::get(&self.db, list_key.as_ref())? {
            None => return Ok(0),
            Some(q) => q
        };

        let mut rem_count = 0u32;

        let tr = self.db.transaction_default();

        if count > 0 { //正向遍历
            let count = count as usize;
            let mut node_key = quick.left().ok_or(Error::none_error("left key"))?.clone();
            let mut node = QuickListNode::get(&tr,node_key.as_ref())?.ok_or(Error::none_error("left node"))?;

            loop {
                let zip_key = node.values_key().ok_or(Error::none_error("zip key"))?.clone();
                let mut zip = ZipList::get(&tr, zip_key.as_ref())?.ok_or(Error::none_error("zip"))?;

                let done = zip.rem((count - rem_count as usize) as i32, value.as_ref());
                rem_count += done;

                if done != 0 {
                    if zip.len() == 0 {
                        //删除当前node
                        tr.delete(zip_key)?;

                        let left = node.left();
                        let right = node.right();

                        match (left,right) {
                            (None,None) => {
                                //都没有数据，删除整个 list
                                tr.delete(&node_key)?;
                                tr.delete(&list_key)?;
                            },
                            (Some(left_key), None) => {
                                let mut left_node = QuickListNode::get(&tr, left_key.as_ref())?.ok_or(Error::none_error("left node"))?;
                                left_node.set_right(&None);
                                tr.delete(node_key)?;
                                tr.put(&list_key, &left_node)?;
                            },
                            (Some(left_key), Some(right_key)) => {
                                let mut left_node = QuickListNode::get(&tr, left_key.as_ref())?.ok_or(Error::none_error("left node"))?;
                                let mut right_node = QuickListNode::get(&tr, right_key.as_ref())?.ok_or(Error::none_error("right node"))?;
                                left_node.set_right(&Some(right_key));
                                right_node.set_right(&Some(left_key));
                                tr.delete(node_key)?;
                                tr.put(left_key, &left_node)?;
                                tr.put(right_key, &right_node)?;
                            },
                            (None, Some(right_key)) => {
                                let mut right_node = QuickListNode::get(&tr, right_key.as_ref())?.ok_or(Error::none_error("right node"))?;
                                right_node.set_left(&None);
                                //todo quick 的right是否要处理
                                quick.set_left(&Some(right_key));
                            },
                        }


                    } else {
                        node.set_len_list(zip.len());
                        node.set_len_bytes(zip.as_ref().len() as LenType);

                        tr.put(zip_key, &zip)?;
                        tr.put(&node_key, &node)?;
                    }
                }

                if rem_count == count as u32 {
                    break
                }
                if let Some(t) = node.right() {
                    node_key = t.clone();
                }else{
                    break
                }
                node = QuickListNode::get(&tr,node_key.as_ref())?.ok_or(Error::none_error("right node"))?;
            }
        }else if count < 0 { //反向遍历
            let count = count.abs() as usize;


        }else{ //正向删除所有相等的值

        }


        if rem_count > 0 {
            quick.set_len_list(quick.len_list() - rem_count);
            tr.put(list_key, quick)?;
        }

        tr.commit()?;
        Ok(rem_count)

    }

    fn ltrim<K: Bytes>(&mut self, key: K, start: i32, stop: i32) -> Result<i32, Error> {
        todo!()
    }

    fn lset<K: Bytes, V: Bytes>(&mut self, key: K, index: i32, value: V) {
        todo!()
    }


    fn rpop<K: Bytes>(&mut self, list_key: K) -> Result<Vec<u8>, Error> {
        let tr = self.db.transaction_default();
        let mut quick = match QuickList::get(&self.db, list_key.as_ref())? {
            None => return Err(Error::not_find("key of list")),
            Some(q) => q
        };
        let node_key = quick.right().ok_or(Error::none_error("right key"))?.clone();
        let mut node = QuickListNode::get(&tr, node_key.as_ref())?.ok_or(Error::none_error("right node"))?;
        let zip_key = node.values_key().ok_or(Error::none_error("zip key"))?.clone();
        let mut zip = ZipList::get(&tr, zip_key.as_ref())?.ok_or(Error::none_error("zip list"))?;
        let value = zip.pop_right();

        if zip.len() == 0 {
            //没有数据，删除quick list node
            if quick.len_node() == 1 {
                tr.delete(zip_key)?;
                tr.delete(node_key)?;
                quick.set_right(&None);
                quick.set_left(&None);
                quick.set_len_list(0);
                quick.set_len_node(0);
                tr.put(list_key.as_ref(), quick)?;
            } else {
                let left = node.left();
                quick.set_right(&node.left());
                quick.set_len_node(quick.len_node() - 1);
                quick.set_len_list(quick.len_list() - 1);
                tr.delete(zip_key)?;
                tr.delete(node_key)?;
                tr.put(list_key.as_ref(), quick)?;
            }
        } else {
            node.set_len_list(zip.len());
            node.set_len_bytes(zip.as_ref().len() as u32);
            quick.set_len_list(quick.len_list() - 1);

            tr.put(zip_key.as_ref(), zip)?;
            tr.put(node_key.as_ref(), node.as_ref())?;
        }

        tr.commit()?;
        Ok(value)
    }

    fn rpoplpush<K: Bytes, V: Bytes>(&mut self, key: K, dstkey: K) -> Result<V, Error> {
        todo!()
    }

    fn rpush<K: Bytes, V: Bytes>(&mut self, list_key: K, value: V) -> Result<i32, Error> {
        let tr = self.db.transaction_default();
        let mut quick = match QuickList::get(&self.db, list_key.as_ref())? {
            None => {
                let mut q = QuickList::new();
                q.init_meta_key(list_key.as_ref());
                q
            }
            Some(q) => q
        };
        let re = quick.rpush(&tr, list_key.as_ref(), value.as_ref())?;
        tr.commit()?;
        Ok(re)
    }

    fn rpush_exists<K: Bytes, V: Bytes>(&mut self, list_key: K, value: V) -> Result<i32, Error> {
        let tr = self.db.transaction_default();
        let mut quick = match QuickList::get(&self.db, list_key.as_ref())? {
            None => return Ok(0),
            Some(q) => q
        };
        let re = quick.rpush(&tr, list_key.as_ref(), value.as_ref())?;
        tr.commit()?;
        Ok(re)
    }
}
