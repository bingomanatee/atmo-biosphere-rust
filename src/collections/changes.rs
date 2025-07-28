/// Test data structures for proof of concept
#[derive(Debug, Clone, PartialEq)]
pub struct FooData {
    pub id: u32,
    pub value: f64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BarData {
    pub id: u32,
    pub energy: f64,
    pub temperature: f64,
}

/// Changes for Foo collection
#[derive(Debug, Clone)]
pub enum FooChange {
    Create(FooData),
    Update { id: u32, value: f64 },
    UpdateName { id: u32, name: String },
    Delete(u32),
}

/// Changes for Bar collection
#[derive(Debug, Clone)]
pub enum BarChange {
    Create(BarData),
    EnergyDelta { id: u32, delta: f64 },
    TemperatureDelta { id: u32, delta: f64 },
    Delete(u32),
}

/// Trait for all change types
pub trait CollectionChange: std::fmt::Debug + Send + Sync {
    /// Get the collection name this change applies to
    fn collection_name(&self) -> &'static str;

    /// Apply this change to a collection (type-erased)
    fn apply_to_collection(&self, collection: &mut dyn std::any::Any) -> Result<(), String>;
}

impl CollectionChange for FooChange {
    fn collection_name(&self) -> &'static str {
        "FOO"
    }

    fn apply_to_collection(&self, collection: &mut dyn std::any::Any) -> Result<(), String> {
        let collection = collection
            .downcast_mut::<crate::collections::Collection<u32, FooData>>()
            .ok_or("Collection type mismatch for FooChange")?;

        match self {
            FooChange::Create(data) => {
                collection.insert(data.id, data.clone());
            },
            FooChange::Update { id, value } => {
                collection.modify(id, |existing| {
                    existing.value = *value;
                });
            },
            FooChange::UpdateName { id, name } => {
                collection.modify(id, |existing| {
                    existing.name = name.clone();
                });
            },
            FooChange::Delete(id) => {
                collection.remove(id);
            },
        }
        Ok(())
    }
}

impl CollectionChange for BarChange {
    fn collection_name(&self) -> &'static str {
        "BAR"
    }

    fn apply_to_collection(&self, collection: &mut dyn std::any::Any) -> Result<(), String> {
        let collection = collection
            .downcast_mut::<crate::collections::Collection<u32, BarData>>()
            .ok_or("Collection type mismatch for BarChange")?;

        match self {
            BarChange::Create(data) => {
                collection.insert(data.id, data.clone());
            },
            BarChange::EnergyDelta { id, delta } => {
                collection.modify(id, |existing| {
                    existing.energy += delta;
                });
            },
            BarChange::TemperatureDelta { id, delta } => {
                collection.modify(id, |existing| {
                    existing.temperature += delta;
                });
            },
            BarChange::Delete(id) => {
                collection.remove(id);
            },
        }
        Ok(())
    }
}
