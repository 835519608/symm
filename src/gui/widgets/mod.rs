//! GUI 控件层：页面只组合参数，样式集中在此维护。

mod button;
mod form;
mod layout;
mod modal;
mod nav;
mod scroll;

pub use button::button;
pub use form::{
    PathBrowse, PathFieldHints, labeled_field, path_field, path_field_with_hints, search_field,
    text_field,
};
pub use layout::{button_row, card, detail_field, empty_hint, form_page, page_heading};
pub use modal::{ModalOptions, ModalSize, show_modal};
pub use nav::settings_nav;
pub use scroll::vertical_when_overflow;
