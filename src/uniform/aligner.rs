use std::u32;
use std::collections::{HashSet, HashMap, BTreeMap};
use std::collections::btree_map::Entry::{Occupied};
use std::net::SocketAddr;

use rand::{thread_rng, sample};


pub(crate) struct Aligner {
    items: BTreeMap<u32, HashSet<SocketAddr>>,
    addrs: HashMap<SocketAddr, u32>,
}


impl Aligner {
    pub fn new() -> Aligner {
        Aligner {
            items: BTreeMap::new(),
            addrs: HashMap::new(),
        }
    }
    pub fn update<N, O>(&mut self, new: N, old: O)
        where N: IntoIterator<Item=SocketAddr>,
              O: IntoIterator<Item=SocketAddr>,
    {
        {
            let zero = self.items.entry(0).or_insert_with(HashSet::new);
            for addr in new {
                let n = *self.addrs.entry(addr).or_insert(0);
                if n == 0 {
                    zero.insert(addr);
                }
            }
        }
        for addr in old {
            if let Some(n) = self.addrs.remove(&addr) {
                match self.items.entry(n) {
                    Occupied(mut o) => {
                        o.get_mut().remove(&addr);
                        if o.get().len() == 0 {
                            o.remove_entry();
                        }
                    }
                    _ => {}
                }
            }
        }
        // in case we inserted it in the first line and did not delete yet
        match self.items.entry(0) {
            Occupied(o) => {
                if o.get().len() == 0 {
                    o.remove_entry();
                }
            }
            _ => {}
        }
    }
    pub fn get<F>(&mut self, limit: u32, blist: F) -> Option<SocketAddr>
        where F: Fn(SocketAddr) -> bool
    {
        assert!(limit < u32::MAX);
        let mut result = None;
        for (&n, addrs) in self.items.iter_mut() {
            if n >= limit {
                return None;
            }
            let mut candidate = sample(&mut thread_rng(),
                addrs.iter().filter(|&&x| !blist(x)).map(|&x| x),
                1);
            match candidate.pop() {
                Some(a) => {
                    addrs.remove(&a);
                    result = Some((n, a));
                    break;
                }
                None => continue,
            }
        }
        if let Some((num, addr)) = result {
            self.items.entry(num+1)
                .or_insert_with(HashSet::new)
                .insert(addr);
            let old = self.addrs.insert(addr, num+1);
            debug_assert_eq!(old, Some(num));
            return Some(addr);
        }
        return None;
    }
    pub fn put(&mut self, addr: SocketAddr) {
        if let Some(num) = self.addrs.get_mut(&addr) {
            assert!(*num > 0);
            match self.items.entry(*num) {
                Occupied(mut o) => {
                    o.get_mut().remove(&addr);
                    if o.get().len() == 0 {
                        o.remove_entry();
                    }
                }
                _ => {}
            }
            *num -= 1;
            self.items.entry(*num).or_insert_with(HashSet::new).insert(addr);
        }
    }
}

#[cfg(test)]
mod test {
    use std::net::{SocketAddr};
    use std::collections::HashMap;
    use super::Aligner;

    fn addr(n: u8) -> SocketAddr {
        SocketAddr::new(format!("127.0.0.{}", n).parse().unwrap(), 80)
    }

    #[test]
    fn normal() {
        let mut a = Aligner::new();
        a.update(vec![addr(1), addr(2), addr(3)], vec![]);

        let mut counter = HashMap::new();
        for _ in 0..30 {
            *counter.entry(a.get(100, |_| false).unwrap()).or_insert(0) += 1;
        }
        assert_eq!(counter, vec![
            (addr(1), 10),
            (addr(2), 10),
            (addr(3), 10),
        ].into_iter().collect::<HashMap<_, _>>());
    }

    #[test]
    fn blacklisting() {
        let mut a = Aligner::new();
        a.update(vec![addr(1), addr(2), addr(3)], vec![]);

        let mut counter = HashMap::new();
        for _ in 0..6 {
            *counter.entry(a.get(100, |_| false).unwrap()).or_insert(0) += 1;
        }
        assert_eq!(counter, vec![
            (addr(1), 2),
            (addr(2), 2),
            (addr(3), 2),
        ].into_iter().collect::<HashMap<_, _>>());

        let blist1 = &|x| x == addr(1);
        for _ in 0..6 {
            *counter.entry(a.get(100, blist1).unwrap()).or_insert(0) += 1;
        }
        assert_eq!(counter, vec![
            (addr(1), 2),
            (addr(2), 5),
            (addr(3), 5),
        ].into_iter().collect::<HashMap<_, _>>());

        for _ in 0..9 {
            *counter.entry(a.get(100, |_| false).unwrap()).or_insert(0) += 1;
        }
        assert_eq!(counter, vec![
            (addr(1), 7),
            (addr(2), 7),
            (addr(3), 7),
        ].into_iter().collect::<HashMap<_, _>>());
    }

    #[test]
    fn update() {
        let mut a = Aligner::new();
        a.update(vec![addr(1), addr(2), addr(3)], vec![]);

        let mut counter = HashMap::new();
        for _ in 0..6 {
            *counter.entry(a.get(100, |_| false).unwrap()).or_insert(0) += 1;
        }
        assert_eq!(counter, vec![
            (addr(1), 2),
            (addr(2), 2),
            (addr(3), 2),
        ].into_iter().collect::<HashMap<_, _>>());

        a.update(vec![addr(4)], vec![addr(2)]);
        for _ in 0..8 {
            *counter.entry(a.get(100, |_| false).unwrap()).or_insert(0) += 1;
        }
        assert_eq!(counter, vec![
            (addr(1), 4),
            (addr(2), 2),  // still here because we count attempts not conns
            (addr(3), 4),
            (addr(4), 4),
        ].into_iter().collect::<HashMap<_, _>>());
    }
}
