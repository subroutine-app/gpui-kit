mod fields;
mod group;
mod group_view;
mod item;
mod list;
mod page;
mod row;
mod section;
mod settings;

pub use fields::*;
pub use group::*;
pub use item::*;
pub use list::*;
pub use page::*;
pub use row::*;
pub use section::SettingSection;
pub use settings::*;

/// Case-insensitive substring search shared by [`Settings`] and [`SettingList`].
fn matches_query(query: &str, text: &str) -> bool {
    text.to_lowercase().contains(&query.to_lowercase())
}
