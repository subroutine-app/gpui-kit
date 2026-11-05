use gpui::App;

mod app_menu_bar;
mod context_menu;
mod context_menu_layer;
mod dropdown_menu;
mod menu_item;
mod popup_menu;

pub use app_menu_bar::{AltTap, AppMenuBar, FocusAppMenuBar};
pub use context_menu::{ContextMenu, ContextMenuExt, ContextMenuState};
pub use context_menu_layer::ContextMenuLayer;
pub use dropdown_menu::DropdownMenu;
pub(crate) use menu_item::MenuItemElement;
pub use popup_menu::{PopupMenu, PopupMenuItem};

pub(crate) fn init(cx: &mut App) {
    app_menu_bar::init(cx);
    popup_menu::init(cx);
}
