pub mod rwlock_serde {
    use serde::de::Deserializer;
    use serde::ser::Serializer;
    use serde::{Deserialize, Serialize};
    use std::sync::RwLock;

    pub fn serialize<S, T>(val: &RwLock<T>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        T::serialize(&*val.read().unwrap(), s)
    }

    pub fn deserialize<'de, D, T>(d: D) -> Result<RwLock<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        Ok(RwLock::new(T::deserialize(d)?))
    }
}

pub mod serde_arena {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use thunderdome::{Arena, Index};

    pub fn serialize<S, T>(val: &Arena<T>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        let arena_serializable: Vec<_> = val
            .iter()
            .map(|(idx, entry)| (idx.to_bits(), entry))
            .collect();

        Vec::<(u64, &T)>::serialize(&arena_serializable, s)
    }

    pub fn deserialize<'de, D, T>(d: D) -> Result<Arena<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        let arena_deserializable = Vec::<(u64, T)>::deserialize(d)?;
        let mut arena = Arena::new();

        for (bits, entry) in arena_deserializable {
            arena.insert_at(Index::from_bits(bits).unwrap(), entry);
        }

        Ok(arena)
    }
}
