#[cfg(feature = "completion")]
pub(crate) mod html_completion;
pub(crate) mod html_folding;
#[cfg(feature = "formatter")]
pub(crate) mod html_formatter;
pub(crate) mod html_highlight;
pub(crate) mod html_hover;
pub(crate) mod html_linked_editing;
pub(crate) mod html_links;
pub(crate) mod html_matching_tag_position;
pub(crate) mod html_rename;
pub(crate) mod html_selection_range;
pub(crate) mod html_symbols;
