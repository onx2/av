use shared::Owner;
use spacetimedb::ReducerContext;

/// A trait for tables that hold ephemeral data that eventually gets persisted.
/// They are expected to have an [Owner] and data column of type [T]
pub trait DataTable<T>: Sized {
    fn find(ctx: &ReducerContext, owner: Owner) -> Option<Self>;
    fn insert(ctx: &ReducerContext, owner: Owner, data: T) -> Self;
    fn delete(&self, ctx: &ReducerContext) -> bool;
    fn update(&self, ctx: &ReducerContext, data: T) -> Self;
}

/// Macro to implement the `DataTable<T>` trait for a SpacetimeDB table row.
///
/// Generates:
/// - `impl DataTable<$data_ty> for $row_ty`
/// - With standard CRUD methods using owner-based access:
///   - `insert(ctx, owner, data)` → inserts new row
///   - `delete(&self, ctx)` → deletes by owner
///   - `update(&self, ctx, new_data)` → updates data, keeps owner
///
/// Assumptions:
/// - Row struct has fields: `owner: Owner`, `data: $data_ty`
/// - Table handle is accessible via `ctx.db.$table_handle()`
/// - Primary key is on `owner`
///
/// Example:
/// ```rust
/// #[table(name = health_tbl)]
/// pub struct Health {
///     #[primary_key]
///     pub owner: Owner,
///     pub data: HealthData,
/// }
///
/// impl_data_table!(
///     table_handle = health_tbl,
///     row         = Health,
///     data        = HealthData
/// );
/// ```
#[macro_export]
macro_rules! impl_data_table {
    (table_handle = $table_handle:ident, row = $row_ty:ty, data = $data_ty:ty) => {
        use $crate::DataTable;
        impl DataTable<$data_ty> for $row_ty {
            fn find(ctx: &spacetimedb::ReducerContext, owner: shared::Owner) -> Option<Self> {
                ctx.db.$table_handle().owner().find(owner)
            }

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
