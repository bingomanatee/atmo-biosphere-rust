use crate::collections::changes::CollectionChange;

/// Factory for creating change objects - completely generic
pub struct ChangeFactory;

impl ChangeFactory {
    /// Create any change that implements CollectionChange
    pub fn create_change<T: CollectionChange + 'static>(change: T) -> Box<dyn CollectionChange> {
        Box::new(change)
    }
}
