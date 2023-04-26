use batbox::{prelude::*, rng::rand::distributions::WeightedError};

pub struct State {
    next_id: usize,
    units: Collection<Unit>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Id(usize);

#[derive(Debug)]
pub struct Attack {
    pub attacker: Id,
    pub target: Id,
}

#[derive(HasId, Debug)]
struct Unit {
    id: Id,
    health: usize,
}

impl State {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            units: default(),
        }
    }

    pub fn spawn(&mut self, health: usize) -> Id {
        let id = Id(self.next_id);
        self.next_id += 1;
        let unit = Unit { id, health };
        self.units.insert(unit);
        id
    }

    pub fn queue(&mut self) -> Option<Attack> {
        if self.units.len() <= 1 {
            return None;
        }
        let units: Vec<Id> = self.units.ids().copied().collect();

        // let target = *units
        //     .choose_weighted(&mut thread_rng(), |id| self.units.get(id).unwrap().health)
        //     .unwrap();
        let target = *units.choose(&mut thread_rng()).unwrap();
        if !thread_rng().gen_bool(
            self.units.get(&target).unwrap().health as f64
                // / self.units.iter().map(|unit| unit.health).sum::<usize>() as f64,
                / self.units.iter().map(|unit| unit.health).max().unwrap() as f64,
        ) {
            return self.queue();
        }

        let target_unit = self.units.get_mut(&target).unwrap();
        target_unit.health -= 1;
        if target_unit.health == 0 {
            self.units.remove(&target);
        }
        Some(Attack {
            target,
            attacker: target,
        })
    }
}

fn winner(healths: impl IntoIterator<Item = usize>) -> usize {
    let mut state = State::new();
    let mut ids = Vec::new();
    for health in healths {
        ids.push(state.spawn(health));
    }
    while let Some(_attack) = state.queue() {
        // ..
    }
    let mut units_left = state.units.into_iter();
    let winner = units_left.next().unwrap();
    assert!(units_left.next().is_none());
    ids.iter().position(|id| id == &winner.id).unwrap()
}

fn win_ratios<const N: usize>(healths: [usize; N]) -> [f64; N] {
    let mut result = [0.0; N];
    const BATTLES: usize = 1_000_000;
    for _ in 0..BATTLES {
        result[winner(healths)] += 1.0;
    }
    result.map(|x| x / BATTLES as f64)
}

#[test]
fn test() {
    let healths = [1, 2, 3, 4];
    assert_eq!(win_ratios(healths), [0.1, 0.2, 0.3, 0.4]);
}
