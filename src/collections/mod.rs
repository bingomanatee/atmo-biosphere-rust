pub mod collections_manager;
pub mod collection;
pub mod change_factory;
pub mod changes;
pub mod tests;
pub mod parallel_example;

pub use collections_manager::CollectionsManager;
pub use collection::Collection;
pub use change_factory::ChangeFactory;
pub use changes::{CollectionChange, FooChange, BarChange};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CollectionsEnum {
    FOO,
    BAR,
}
