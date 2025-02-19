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

pub mod serde_thunderdome_index {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use thunderdome::Index;

    pub fn serialize<S>(val: &Index, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        u64::serialize(&val.to_bits(), s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Index, D::Error>
    where
        D: Deserializer<'de>,
    {
        let deserializable = u64::deserialize(d)?;

        Ok(Index::from_bits(deserializable).unwrap())
    }
}
