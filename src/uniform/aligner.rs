use std::u32;
use std::collections::{HashSet, BTreeMap};
use std::net::SocketAddr;

use rand::{thread_rng, sample};


pub(crate) struct Aligner {
    items: BTreeMap<u32, HashSet<SocketAddr>>,
}


impl Aligner {
    pub fn new() -> Aligner {
        Aligner {
            items: BTreeMap::new(),
        }
    }
    pub fn add<I: IntoIterator<Item=SocketAddr>>(&mut self, iter: I) {
        let mut new: HashSet<_> = iter.into_iter().collect();
        for (_, addrs) in &self.items {
            for a in addrs {
                new.remove(a);
            }
        }
        self.items.entry(0)
            .or_insert_with(HashSet::new)
            .extend(&new);
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
            return Some(addr);
        }
        return None;
    }
}

#[cfg(test)]
mod test {
    use std::net::{SocketAddr, IpAddr};
    use std::collections::HashMap;
    use super::Aligner;

    fn addr(n: u8) -> SocketAddr {
        SocketAddr::new(format!("127.0.0.{}", n).parse().unwrap(), 80)
    }

    #[test]
    fn normal() {
        let mut a = Aligner::new();
        a.add(vec![addr(1), addr(2), addr(3)]);

        let mut counter = HashMap::new();
        for _ in 0..30 {
            println!("A {:?}", a.items);
            *counter.entry(a.get(100, |x| false).unwrap()).or_insert(0) += 1;
        }
        assert_eq!(counter, vec![
            (addr(1), 10),
            (addr(2), 10),
            (addr(3), 10),
        ].into_iter().collect::<HashMap<_, _>>());

    }
}
