//! 无 CLI 参数时的记录选择（查库在 workflow，菜单在 ui）。

use crate::domain::error::SymmError;
use crate::domain::model::LinkRecord;
use crate::ui::interaction::pick_record;
use crate::workflows::pick_list;
use crate::workflows::selector;

pub fn pick_one_selector(conn: &rusqlite::Connection) -> Result<String, SymmError> {
    let entries = pick_list::list_entries(conn)?;
    if entries.is_empty() {
        return Err(SymmError::NotFound {
            selector: "(空库)".to_string(),
        });
    }
    if entries.is_truncated() {
        return pick_record::prompt_one_selector(entries.total(), entries.option_limit());
    }
    let options: Vec<String> = entries.iter().map(pick_list::format_label).collect();
    let selected = pick_record::pick_one_option(&options)?;
    let index =
        pick_list::parse_label_index(&selected).ok_or_else(|| SymmError::InvalidArgument {
            message: "无法识别所选记录".to_string(),
        })?;
    Ok(index.to_string())
}

pub fn pick_many_records(conn: &rusqlite::Connection) -> Result<Vec<LinkRecord>, SymmError> {
    let entries = pick_list::list_entries(conn)?;
    if entries.is_empty() {
        return Err(SymmError::NotFound {
            selector: "(空库)".to_string(),
        });
    }
    if entries.is_truncated() {
        let selectors =
            pick_record::prompt_many_selectors(entries.total(), entries.option_limit())?;
        return records_from_selectors(conn, &selectors);
    }
    let options: Vec<String> = entries.iter().map(pick_list::format_label).collect();
    let selected = pick_record::pick_many_options(&options)?;
    if selected.is_empty() {
        return Err(SymmError::InvalidArgument {
            message: "未选择任何记录".to_string(),
        });
    }
    pick_list::records_for_labels(&entries, &selected)
}

fn records_from_selectors(
    conn: &rusqlite::Connection,
    selectors: &[String],
) -> Result<Vec<LinkRecord>, SymmError> {
    let picked = selector::records_from_tokens(conn, selectors)?;
    if picked.is_empty() {
        return Err(SymmError::InvalidArgument {
            message: "未选择任何记录".to_string(),
        });
    }
    Ok(picked)
}
