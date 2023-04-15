pub mod node;

use node::Node;
use serde::{Deserialize, Serialize};
use thunderdome::{Arena, Index};

use self::node::NodeEvent;

#[derive(Serialize, Deserialize)]
struct Entry {
    #[serde(with = "vec_opt_idx")]
    inputs: Vec<Option<Index>>,
    node: Box<dyn Node>,
}

impl Clone for Entry {
    fn clone(&self) -> Self {
        Entry {
            inputs: self.inputs.clone(),
            node: dyn_clone::clone_box(&*self.node),
        }
    }
}

mod vec_opt_idx {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use thunderdome::Index;

    pub fn serialize<S>(val: &Vec<Option<Index>>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let serializable: Vec<_> = val.iter().map(|opt| opt.map(|idx| idx.to_bits())).collect();

        Vec::<Option<u64>>::serialize(&serializable, s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Vec<Option<Index>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let deserializable = Vec::<Option<u64>>::deserialize(d)?;

        Ok(deserializable
            .iter()
            .map(|opt| opt.map(|bits| Index::from_bits(bits).unwrap()))
            .collect())
    }
}

impl Entry {
    fn new(inputs: Vec<Option<Index>>, node: Box<dyn Node>) -> Self {
        Entry { inputs, node }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Runtime {
    #[serde(skip)]
    values: Arena<f32>,
    #[serde(with = "crate::util::serde_arena")]
    nodes: Arena<Entry>,
}

impl Runtime {
    pub fn new() -> Self {
        Runtime {
            values: Arena::new(),
            nodes: Arena::new(),
        }
    }

    pub fn insert(
        &mut self,
        inputs: impl Into<Vec<Option<Index>>>,
        node: Box<dyn Node + 'static>,
    ) -> Index {
        let inputs = inputs.into();
        assert_eq!(inputs.len(), node.inputs().len());
        self.nodes.insert(Entry::new(inputs, node))
    }

    pub fn remove(&mut self, index: Index) {
        self.nodes.remove(index);
    }

    pub fn set_input(&mut self, index: Index, port: usize, new_input: Option<Index>) {
        self.nodes[index].inputs[port] = new_input;
    }

    pub fn set_all_inputs(&mut self, index: Index, new_inputs: Vec<Option<Index>>) {
        self.nodes[index].inputs = new_inputs;
    }

    pub fn step(&mut self) -> Vec<(Index, Vec<NodeEvent>)> {
        let mut evs = Vec::new();
        let mut buf = Vec::new();

        self.values.clear();

        for (idx, entry) in &self.nodes {
            self.values.insert_at_slot(idx.slot(), entry.node.read());
        }

        for (idx, entry) in &mut self.nodes {
            buf.clear();
            for &mut input in &mut entry.inputs {
                buf.push(match input {
                    Some(in_index) => Some(*self.values.get_by_slot(in_index.slot()).unwrap().1),
                    None => None,
                });
            }

            let evs_one = entry.node.feed(&buf);
            evs.push((idx, evs_one));
        }

        evs
    }

    pub fn peek(&self, index: Index) -> f32 {
        self.values
            .get_by_slot(index.slot())
            .map(|(_idx, val)| *val)
            .unwrap_or(0.0)
    }
}
