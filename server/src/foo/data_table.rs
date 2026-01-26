use shared::Owner;
use spacetimedb::ReducerContext;

/// A trait for tables that hold ephemeral data that eventually gets persisted.
/// They are expected to have an [Owner] and data column of type [T]
pub trait DataTable<T> {
    fn insert(ctx: &ReducerContext, owner: Owner, data: T) -> Self;
    fn delete(&self, ctx: &ReducerContext) -> bool;
    fn update(&self, ctx: &ReducerContext, data: T) -> Self;
}

/// Used to implement basic CRUD actions for this [DataTable]
#[macro_export]
macro_rules! impl_data_table {
    (table_handle = $table_handle:ident, row = $row_ty:ty, data = $data_ty:ty) => {
        use super::DataTable;
        impl DataTable<$data_ty> for $row_ty {
            fn insert(
                ctx: &spacetimedb::ReducerContext,
                owner: shared::Owner,
                data: $data_ty,
            ) -> Self {
                spacetimedb::Table::insert(ctx.db.$table_handle(), Self { owner, data })
            }

            fn delete(&self, ctx: &spacetimedb::ReducerContext) -> bool {
                ctx.db.$table_handle().owner().delete(self.owner)
            }

            fn update(&self, ctx: &spacetimedb::ReducerContext, data: $data_ty) -> Self {
                ctx.db.$table_handle().owner().update(Self {
                    owner: self.owner,
                    data,
                })
            }
        }
    };
}
