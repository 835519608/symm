//! GUI 控件层：页面只组合参数，样式集中在此维护。

mod button;
mod form;
mod layout;
mod modal;
mod nav;
mod scroll;
mod selection;
mod slider;

pub use button::button;
pub use form::{PathBrowse, PathPickMode, path_control_row, path_field, search_field, text_field};
pub use layout::{
    card, detail_field, detail_path_field, right_aligned, settings_content_frame, split_row,
};
pub use modal::{
    ModalOptions, ModalSection, ModalSize, fill_ui_width, modal_scroll_vertical, show_modal,
};
pub use nav::settings_nav;
pub use scroll::vertical_when_overflow;
pub use selection::{
    SelectableTextStyle, checkbox_icon, radio_value, selectable_row_rect, selectable_text_row,
};
pub use slider::value_slider;
