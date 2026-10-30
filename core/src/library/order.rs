//! Each tab's sort order, `sort.json` (since 0.2): `{"<category id>":
//! "<order>"}`. A tab not in the file is in name order, A to Z.

use serde_json::{Map, Value};

use super::played::{Play, SystemDates};

pub const SORT_FILE: &str = "sort.json";

/// How a tab orders its titles.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortOrder {
    /// The host's order: by name, as the scan lists the titles.
    NameAsc,
    NameDesc,
    /// The later of the system's last launch and Pocket Shelf's.
    RecentlyPlayed,
    LeastRecentlyPlayed,
    /// Launches through Pocket Shelf (the system keeps no count).
    MostPlayed,
    LeastPlayed,
    /// The system's first-install date.
    NewestInstalled,
    OldestInstalled,
}

impl SortOrder {
    /// In the Sort row's order.
    pub const ALL: [SortOrder; 8] = [
        SortOrder::NameAsc,
        SortOrder::NameDesc,
        SortOrder::RecentlyPlayed,
        SortOrder::LeastRecentlyPlayed,
        SortOrder::MostPlayed,
        SortOrder::LeastPlayed,
        SortOrder::NewestInstalled,
        SortOrder::OldestInstalled,
    ];

    pub fn key(self) -> &'static str {
        match self {
            SortOrder::NameAsc => "name",
            SortOrder::NameDesc => "name-desc",
            SortOrder::RecentlyPlayed => "played-recent",
            SortOrder::LeastRecentlyPlayed => "played-oldest",
            SortOrder::MostPlayed => "played-most",
            SortOrder::LeastPlayed => "played-least",
            SortOrder::NewestInstalled => "installed-newest",
            SortOrder::OldestInstalled => "installed-oldest",
        }
    }

    pub fn from_key(key: &str) -> Option<SortOrder> {
        SortOrder::ALL.into_iter().find(|o| o.key() == key)
    }
}

/// Each tab's order; missing tabs are `NameAsc`.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct SortOrders(Vec<(String, SortOrder)>);

impl SortOrders {
    pub fn parse(bytes: &[u8]) -> SortOrders {
        let Ok(Value::Object(map)) = serde_json::from_slice::<Value>(bytes) else { return SortOrders::default() };
        SortOrders(
            map.iter()
                .filter_map(|(id, v)| Some((id.clone(), SortOrder::from_key(v.as_str()?)?)))
                .filter(|(_, order)| *order != SortOrder::NameAsc)
                .collect(),
        )
    }

    /// Two-space JSON and a newline, tabs in id order.
    pub fn render(&self) -> String {
        let mut sorted: Vec<&(String, SortOrder)> = self.0.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        let map: Map<String, Value> = sorted.into_iter().map(|(id, order)| (id.clone(), order.key().into())).collect();
        serde_json::to_string_pretty(&Value::Object(map)).expect("json") + "\n"
    }

    pub fn get(&self, id: &str) -> SortOrder {
        self.0.iter().find(|(tab, _)| tab == id).map_or(SortOrder::NameAsc, |(_, order)| *order)
    }

    pub fn set(&mut self, id: &str, order: SortOrder) {
        self.0.retain(|(tab, _)| tab != id);
        if order != SortOrder::NameAsc {
            self.0.push((id.to_string(), order));
        }
    }
}

/// What a title's place in a played or installed order is worked out from.
pub struct Facts {
    pub play: Option<Play>,
    pub system: Option<SystemDates>,
}

impl Facts {
    /// The later of the system's last launch and Pocket Shelf's.
    fn last_played(&self) -> Option<u64> {
        let own = self.play.map(|p| p.last).filter(|&t| t > 0);
        let system = self.system.and_then(|s| s.launched);
        own.max(system)
    }
}

/// `list` (a tab's titles in name order) in `order`. Ties and titles with
/// nothing to sort by keep name order: a title never played counts as the
/// least recently and least played, one with no install date goes last in
/// both install orders.
pub fn sort(list: &mut [usize], order: SortOrder, facts: impl Fn(usize) -> Facts) {
    match order {
        SortOrder::NameAsc => {}
        SortOrder::NameDesc => list.reverse(),
        SortOrder::RecentlyPlayed | SortOrder::LeastRecentlyPlayed => {
            let mut keyed: Vec<(Option<u64>, usize)> = list.iter().map(|&i| (facts(i).last_played(), i)).collect();
            if order == SortOrder::RecentlyPlayed {
                keyed.sort_by(|a, b| b.0.cmp(&a.0));
            } else {
                keyed.sort_by(|a, b| a.0.cmp(&b.0));
            }
            write_back(list, keyed);
        }
        SortOrder::MostPlayed | SortOrder::LeastPlayed => {
            let mut keyed: Vec<(u32, usize)> = list.iter().map(|&i| (facts(i).play.map_or(0, |p| p.count), i)).collect();
            if order == SortOrder::MostPlayed {
                keyed.sort_by(|a, b| b.0.cmp(&a.0));
            } else {
                keyed.sort_by(|a, b| a.0.cmp(&b.0));
            }
            write_back(list, keyed);
        }
        SortOrder::NewestInstalled | SortOrder::OldestInstalled => {
            let mut keyed: Vec<(Option<u64>, usize)> = list.iter().map(|&i| (facts(i).system.and_then(|s| s.installed), i)).collect();
            let newest = order == SortOrder::NewestInstalled;
            keyed.sort_by(|a, b| match (a.0, b.0) {
                (Some(x), Some(y)) if newest => y.cmp(&x),
                (Some(x), Some(y)) => x.cmp(&y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            });
            write_back(list, keyed);
        }
    }
}

fn write_back<K>(list: &mut [usize], keyed: Vec<(K, usize)>) {
    for (slot, (_, index)) in list.iter_mut().zip(keyed) {
        *slot = index;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orders_round_trip_and_name_order_is_left_out() {
        let mut orders = SortOrders::default();
        orders.set("games", SortOrder::RecentlyPlayed);
        orders.set("retro-snes", SortOrder::NameDesc);
        orders.set("psp", SortOrder::NameAsc);
        assert_eq!(orders.render(), "{\n  \"games\": \"played-recent\",\n  \"retro-snes\": \"name-desc\"\n}\n");
        assert_eq!(SortOrders::parse(orders.render().as_bytes()), orders);
        assert_eq!(orders.get("psp"), SortOrder::NameAsc);
        orders.set("games", SortOrder::NameAsc);
        assert_eq!(orders.get("games"), SortOrder::NameAsc);
        assert_eq!(SortOrders::parse(br#"{"games":"sideways"}"#), SortOrders::default());
        assert_eq!(SortOrders::parse(b"nope"), SortOrders::default());
    }

    #[test]
    fn each_order_puts_titles_without_data_where_it_says() {
        // Titles 0..5 in name order. 1 and 3 played (3 later, 1 more often),
        // 2 launched by the system only; 0 and 4 never. 0 installed first,
        // 4 last, 1 has no date.
        let facts = |i: usize| Facts {
            play: match i {
                1 => Some(Play { count: 5, last: 100 }),
                3 => Some(Play { count: 2, last: 300 }),
                _ => None,
            },
            system: Some(SystemDates {
                launched: if i == 2 { Some(200) } else { None },
                installed: match i {
                    0 => Some(10),
                    2 => Some(20),
                    3 => Some(30),
                    4 => Some(40),
                    _ => None,
                },
            }),
        };
        let sorted = |order| {
            let mut list = vec![0, 1, 2, 3, 4];
            sort(&mut list, order, facts);
            list
        };
        assert_eq!(sorted(SortOrder::NameAsc), [0, 1, 2, 3, 4]);
        assert_eq!(sorted(SortOrder::NameDesc), [4, 3, 2, 1, 0]);
        assert_eq!(sorted(SortOrder::RecentlyPlayed), [3, 2, 1, 0, 4]);
        assert_eq!(sorted(SortOrder::LeastRecentlyPlayed), [0, 4, 1, 2, 3]);
        assert_eq!(sorted(SortOrder::MostPlayed), [1, 3, 0, 2, 4]);
        assert_eq!(sorted(SortOrder::LeastPlayed), [0, 2, 4, 3, 1]);
        assert_eq!(sorted(SortOrder::NewestInstalled), [4, 3, 2, 0, 1]);
        assert_eq!(sorted(SortOrder::OldestInstalled), [0, 2, 3, 4, 1]);
    }
}
