/**
 * A FileChooser widget for easy-imgui.
 *
 * This widget does not create a window or a popup. It is up to you to create it in a
 * proper place.
 */
use bytesize::ByteSize;
use easy_imgui::{self as imgui, id, lbl, lbl_id, CustomRectIndex, HasImGuiContext};
pub use glob::{self, Pattern};
use image::DynamicImage;
use std::io::Result;
use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    fs::DirEntry,
    path::{Path, PathBuf},
    time::SystemTime,
};
use time::macros::format_description;

#[cfg(feature = "tr")]
include!(concat!(env!("OUT_DIR"), "/locale/translators.rs"));

/// Sets the language for this widget
#[cfg(feature = "tr")]
pub fn set_locale(locale: &str) {
    translators::set_locale(locale);
}

/// Sets the language for this widget
#[cfg(not(feature = "tr"))]
pub fn set_locale(_locale: &str) {}

#[cfg(feature = "tr")]
use tr::tr;

#[cfg(not(feature = "tr"))]
macro_rules! tr {
    ($($args:tt)*) => { format!($($args)*) };
}

/// Main widget to create a file chooser.
///
/// Create one of these when the widget is opened, and then call `do_ui` for each frame.
pub struct FileChooser {
    path: PathBuf,
    flags: Flags,
    entries: Vec<FileEntry>,
    selected: Option<usize>,
    sort_dirty: bool,
    visible_dirty: bool,
    scroll_dirty: bool,
    show_hidden: bool,
    popup_dirs: Vec<(PathBuf, bool)>,
    search_term: String,
    file_name: OsString,
    // Index in `filters`. If that is empty, this is unused.
    active_filter_idx: usize,
    filters: Vec<Filter>,
    read_only: bool,
    visible_entries: Vec<usize>,
    path_size_overflow: f32,
}

/// The output of calling `do_ui` each frame.
pub enum Output {
    /// The widget is still opened.
    ///
    /// Usually you will keep calling `do_ui` unless you have other means of closing it.
    Continue,
    /// The widget wants to be closed, or failed scanning the directory.
    ///
    /// You could ignore it, but usually there is no reason to do that.
    Cancel,
    /// The widget wants to accept a file.
    ///
    /// You can check if the selection is acceptable and decide to close or not close it
    /// as you see appropriate.
    Ok,
}

impl std::fmt::Debug for FileChooser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileChooser")
            .field("path", &self.path())
            .field("file_name", &self.file_name())
            .field("active_filter", &self.active_filter())
            .field("read_only", &self.read_only())
            .finish()
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum FileEntryKind {
    Parent,
    Directory,
    File,
    Root, // drive letter
}

struct FileEntry {
    name: OsString,
    kind: FileEntryKind,
    size: Option<u64>,
    modified: Option<SystemTime>,
    hidden: bool,
}

/// An identifier for the filter.
///
/// This is given back in `OutputOk` so you can identify the active filter,
/// if you need it.
/// You can use the same value for several filters, if you want.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct FilterId(pub i32);

/// An entry in the "File filter" combo box.
#[derive(Default, Debug, Clone)]
pub struct Filter {
    /// The identifier of the filter.
    ///
    /// That of the active filter will be returned in `OutputOk`.
    pub id: FilterId,
    /// The text shown in the combo box.
    pub text: String,
    /// A list of glob patterns for this filter.
    ///
    /// An empty list means "any file".
    /// Normal patterns are of the form `"*.txt"`, but you may write anything you want.
    /// They are case-insensitive.
    pub globs: Vec<glob::Pattern>,
}

impl Filter {
    /// Checkes wether a file name matches this filter.
    pub fn matches(&self, name: impl AsRef<OsStr>) -> bool {
        let name = name.as_ref().to_string_lossy();
        let opts = glob::MatchOptions {
            case_sensitive: false,
            ..Default::default()
        };
        // And empty globs list equals "*", ie, everything.
        self.globs.is_empty() || self.globs.iter().any(|glob| glob.matches_with(&name, opts))
    }
}

#[cfg(target_os = "windows")]
mod os {
    use std::path::PathBuf;

    pub struct Roots {
        drives: u32,
        letter: u8,
    }

    impl Roots {
        pub fn new() -> Roots {
            let drives = unsafe { windows::Win32::Storage::FileSystem::GetLogicalDrives() };
            Roots { drives, letter: 0 }
        }
    }
    impl Iterator for Roots {
        type Item = PathBuf;

        fn next(&mut self) -> Option<PathBuf> {
            while self.letter < 26 {
                // A .. Z
                let bit = 1 << self.letter;
                let drive = char::from(b'A' + self.letter);
                self.letter += 1;
                if (self.drives & bit) != 0 {
                    return Some(PathBuf::from(format!("{}:\\", drive)));
                }
            }
            None
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod os {
    use std::path::PathBuf;

    pub struct Roots(());

    impl Roots {
        pub fn new() -> Roots {
            Roots(())
        }
    }

    impl Iterator for Roots {
        type Item = PathBuf;

        // We could return '/' here, that is a root, but it looks quite useless as a choice.
        fn next(&mut self) -> Option<PathBuf> {
            None
        }
    }
}

struct EnumSubdirs {
    dir: std::fs::ReadDir,
}

impl EnumSubdirs {
    fn new(path: impl Into<PathBuf>) -> Result<EnumSubdirs> {
        let path = path.into();
        let dir = std::fs::read_dir(path)?;
        Ok(EnumSubdirs { dir })
    }
}

impl Iterator for EnumSubdirs {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = self.dir.next()?.ok()?;
            let path = next.path();
            let meta = path.metadata().ok()?;
            let is_dir = meta.is_dir();
            if !is_dir {
                continue;
            }
            return Some(path);
        }
    }
}

bitflags::bitflags! {
    pub struct Flags: u32 {
        /// Shows the "Read only" check.
        const SHOW_READ_ONLY = 1;
    }
}

impl Default for FileChooser {
    fn default() -> Self {
        FileChooser::new()
    }
}

impl FileChooser {
    /// Creates a new default widget.
    pub fn new() -> FileChooser {
        FileChooser {
            path: PathBuf::default(),
            flags: Flags::empty(),
            entries: Vec::new(),
            selected: None,
            sort_dirty: false,
            visible_dirty: false,
            scroll_dirty: false,
            show_hidden: false,
            popup_dirs: Vec::new(),
            search_term: String::new(),
            file_name: OsString::new(),
            active_filter_idx: 0,
            filters: Vec::new(),
            read_only: false,
            visible_entries: Vec::new(),
            path_size_overflow: 0.0,
        }
    }
    /// Adds the given option flags.
    pub fn add_flags(&mut self, flags: Flags) {
        self.flags |= flags;
    }
    /// Removes the given option flags.
    pub fn remove_flags(&mut self, flags: Flags) {
        self.flags &= !flags;
    }
    /// Changes the current visible directory.
    ///
    /// By default it will be the curent working directory (".").
    pub fn set_path(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = std::path::absolute(path)?;
        self.selected = None;
        // Reuse the entries memory
        let mut entries = std::mem::take(&mut self.entries);
        entries.clear();
        let mut add_entry = |entry: FileEntry| {
            if self.file_name == entry.name {
                self.selected = Some(entries.len());
            }
            entries.push(entry);
        };
        if path.parent().is_some() {
            add_entry(FileEntry::dot_dot());
        }
        for rd in std::fs::read_dir(&path)? {
            let Ok(rd) = rd else { continue };
            add_entry(FileEntry::new(&rd));
        }
        self.path = path;
        self.entries = entries;
        self.sort_dirty = true;
        self.visible_dirty = true;
        self.scroll_dirty = true;
        self.popup_dirs.clear();
        self.search_term.clear();
        self.path_size_overflow = 0.0;
        Ok(())
    }

    /// Changes the typed file name.
    ///
    /// By default it is empty.
    pub fn set_file_name(&mut self, file_name: impl AsRef<OsStr>) {
        self.file_name = file_name.as_ref().to_owned();

        if !self.file_name.is_empty() {
            // Select the better matching filter
            for (i_f, f) in self.filters.iter().enumerate() {
                if f.matches(&self.file_name) {
                    self.active_filter_idx = i_f;
                    break;
                }
            }
            // Select the better matching entry
            for (i_entry, entry) in self.entries.iter().enumerate() {
                if entry.name == self.file_name {
                    self.selected = Some(i_entry);
                    break;
                }
            }
        }
    }

    /// Gets the current showing directory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Gets the current file name.
    ///
    /// To get the final selection it is better to use the `OutputOk` struct.
    /// This is more useful for interactive things, such as previews.
    pub fn file_name(&self) -> &OsStr {
        // If the selected item maches the typed name (not counting lossy bits), then
        // use the original entry.name, that will better represent the original OsString,
        // and likely the user intent.
        // This is important only if the file has non-UTF-8 sequences.
        if let Some(i_sel) = self.selected {
            let entry = &self.entries[i_sel];
            if entry.kind == FileEntryKind::File && self.file_name == *entry.name.to_string_lossy()
            {
                return &entry.name;
            }
        }
        &self.file_name
    }
    /// Gets the current active filter, if any.
    /// It is None only if no filters have been added.
    pub fn active_filter(&self) -> Option<FilterId> {
        if self.filters.is_empty() {
            None
        } else {
            Some(self.filters[self.active_filter_idx].id)
        }
    }
    /// Gets the status of the read-only check box.
    /// If the SHOW_READ_ONLY flag is not specified, it will return `false`.
    pub fn read_only(&self) -> bool {
        self.read_only
    }
    /// Combine `path + file_name` and optionally an extension.
    ///
    /// The `default_extension` will only be used if `file_name` has no extension
    /// of its own, and it doesn't exist in disk. This is useful if you want to set
    /// a default extension depending on the active filter.
    pub fn full_path(&self, default_extension: Option<&str>) -> PathBuf {
        let file_name = self.file_name();
        let mut res = self.path.join(file_name);
        if let (None, Some(new_ext)) = (res.extension(), default_extension) {
            if !res.exists() {
                res.set_extension(new_ext);
            }
        }
        res
    }

    #[cfg(target_os = "windows")]
    fn set_path_super_root(&mut self) {
        self.path = PathBuf::default();
        self.entries = os::Roots::new()
            .map(|path| FileEntry {
                name: path.into(),
                kind: FileEntryKind::Root,
                size: None,
                modified: None,
                hidden: false,
            })
            .collect();
        self.selected = None;
        self.sort_dirty = true;
        self.visible_dirty = true;
        self.scroll_dirty = true;
        self.popup_dirs.clear();
        self.search_term.clear();
    }

    /// Adds a filter to the list of filters.
    pub fn add_filter(&mut self, filter: Filter) {
        self.filters.push(filter);
        if !self.file_name.is_empty()
            && !self.filters[self.active_filter_idx].matches(&self.file_name)
        {
            // The filter just inserted
            if self.filters.last().unwrap().matches(&self.file_name) {
                self.active_filter_idx = self.filters.len() - 1;
            }
        }
        self.visible_dirty = true;
    }
    /// Draws the widget in the current frame.
    ///
    /// `params` is a `UiParameters` value that contains additional parameters for the UI.
    /// The only mandatory parameter is the `CustomAtlas`. If you just want this one, you
    /// can pass a `&CustomAtlas` directly.
    pub fn do_ui<'a, A, Params, Preview>(&mut self, ui: &'a imgui::Ui<A>, params: Params) -> Output
    where
        Params: Into<UiParameters<'a, Preview>>,
        Preview: PreviewBuilder<A>,
    {
        if self.entries.is_empty() {
            let res = self.set_path(".");
            if res.is_err() {
                return Output::Cancel;
            }
        }

        let UiParameters { atlas, mut preview } = params.into();
        let mut next_path = None;
        let mut output = Output::Continue;

        let mut my_path = PathBuf::new();

        let style = ui.style();
        ui.child_config(lbl("path"))
            //.child_flags(imgui::ChildFlags::AutoResizeY)
            .size(imgui::Vector2::new(
                0.0,
                ui.get_frame_height_with_spacing()
                    + if self.path_size_overflow < 0.0 {
                        style.ScrollbarSize
                    } else {
                        0.0
                    },
            ))
            .window_flags(imgui::WindowFlags::HorizontalScrollbar)
            .with(|| {
                #[cfg(target_os = "windows")]
                {
                    let scale = ui.get_font_size() / 16.0;
                    if ui
                        .image_button_with_custom_rect_config(id("::super"), atlas.mypc_rr, scale)
                        .build()
                    {
                        self.set_path_super_root();
                    }
                    ui.same_line();
                }

                let mut my_disk = None;
                'component: for component in self.path.components() {
                    let piece = 'piece: {
                        // This is tricky because in normal OSes the root is just '/', but
                        // in some weird ones, there are multiple roots ('C:\', 'D:\'). We'll just ignore everything except drive letters.
                        match component {
                            std::path::Component::Prefix(prefix) => match prefix.kind() {
                                std::path::Prefix::VerbatimDisk(disk)
                                | std::path::Prefix::Disk(disk) => {
                                    my_disk = Some(char::from(disk));
                                    continue 'component;
                                }
                                _ => (),
                            },
                            std::path::Component::RootDir => {
                                if let Some(my_disk) = my_disk.take() {
                                    let drive_root = format!(
                                        "{}:{}",
                                        my_disk,
                                        component.as_os_str().to_string_lossy()
                                    );
                                    let drive_root = OsString::from(drive_root);
                                    break 'piece Cow::Owned(drive_root);
                                }
                            }
                            _ => (),
                        }
                        Cow::Borrowed(component.as_os_str())
                    };

                    my_path.push(&piece);
                    if ui
                        .button_config(lbl_id(piece.to_string_lossy(), my_path.to_string_lossy()))
                        .build()
                    {
                        next_path = Some(my_path.clone());
                    }
                    if ui.is_mouse_released(imgui::MouseButton::Right)
                        && ui.is_item_hovered_ex(imgui::HoveredFlags::AllowWhenBlockedByPopup)
                    {
                        let mut popup_dirs = Vec::new();
                        if let Some(parent) = my_path.parent() {
                            if let Ok(subdir) = EnumSubdirs::new(parent) {
                                for d in subdir {
                                    if d.file_name().is_some() {
                                        let sel = d == my_path;
                                        popup_dirs.push((d, sel));
                                    }
                                }
                            }
                        } else {
                            for root in os::Roots::new() {
                                let sel = root == my_path;
                                popup_dirs.push((root, sel));
                            }
                        }
                        popup_dirs.sort_by(|a, b| a.0.cmp(&b.0));
                        self.popup_dirs = popup_dirs;
                        if !self.popup_dirs.is_empty() {
                            ui.open_popup(id(c"popup_dirs"));
                        }
                    }
                    ui.same_line();
                }
                // If the length of the components has changed, scroll to the end, and record the
                // size overflow.
                let path_size_overflow = ui.get_content_region_avail().x;
                if path_size_overflow != self.path_size_overflow {
                    ui.set_scroll_here_x(1.0);
                    self.path_size_overflow = path_size_overflow;
                }

                ui.popup_config(id(c"popup_dirs")).with(|| {
                    for (dir, sel) in &self.popup_dirs {
                        let name = dir.file_name().unwrap_or_else(|| dir.as_os_str());
                        if ui
                            .selectable_config(lbl_id(
                                name.to_string_lossy(),
                                dir.display().to_string(),
                            ))
                            .selected(*sel)
                            .build()
                        {
                            next_path = Some(dir.clone());
                        }
                    }
                });
            });

        ui.text(&tr!("Search"));
        ui.same_line();
        ui.set_next_item_width(-ui.get_frame_height_with_spacing() - style.ItemSpacing.x);
        if ui
            .input_text_config(lbl_id(c"", c"Search"), &mut self.search_term)
            .build()
        {
            self.visible_dirty = true;
        }

        ui.same_line();
        ui.with_push(
            self.show_hidden.then_some(((
                imgui::ColorId::Button,
                style.color(imgui::ColorId::ButtonActive),
            ),)),
            || {
                let scale = ui.get_font_size() / 16.0;
                if ui
                    .image_button_with_custom_rect_config(id("hidden"), atlas.hidden_rr, scale)
                    .build()
                {
                    self.show_hidden ^= true;
                    self.visible_dirty = true;
                }
            },
        );

        let style = ui.style();
        // Two rows of full controls
        let reserve = 2.0 * ui.get_frame_height_with_spacing();
        let preview_width = preview.width();
        ui.table_config(lbl("FileChooser"), 4)
            .flags(
                imgui::TableFlags::RowBg
                    | imgui::TableFlags::ScrollY
                    | imgui::TableFlags::Resizable
                    | imgui::TableFlags::Sortable
                    | imgui::TableFlags::SizingFixedFit,
            )
            .outer_size(imgui::Vector2::new(-preview_width, -reserve))
            .with(|| {
                let pad = ui.style().FramePadding;
                ui.table_setup_column("", imgui::TableColumnFlags::None, 0.00, 0);
                ui.table_setup_column(
                    tr!("Name"),
                    imgui::TableColumnFlags::WidthStretch | imgui::TableColumnFlags::DefaultSort,
                    0.0,
                    0,
                );
                ui.table_setup_column(
                    tr!("Size"),
                    imgui::TableColumnFlags::WidthFixed,
                    ui.calc_text_size("999.9 GiB").x + 2.0 * pad.x,
                    0,
                );
                ui.table_setup_column(
                    tr!("Modified"),
                    imgui::TableColumnFlags::WidthFixed,
                    ui.calc_text_size("2024-12-31 23:59:59").x + 2.0 * pad.x,
                    0,
                );
                ui.table_setup_scroll_freeze(0, 1);
                ui.table_headers_row();

                // First we sort the entries in-place, then we filter them into `visible_entries`.
                // We could do it the other way around, and it might be more efficient some times,
                // but it probably doesn't matter too much in practice.

                ui.table_with_sort_specs_always(|dirty, specs| {
                    if dirty || self.sort_dirty {
                        self.sort_dirty = false;
                        self.resort_entries(specs);
                    }
                    false
                });
                if self.visible_dirty {
                    self.visible_dirty = false;
                    self.scroll_dirty = true;
                    self.recompute_visible_entries();
                }

                let mut clipper = ui.list_clipper(self.visible_entries.len());
                // If `scroll_dirty` we have to move the scroll to the "best" place.
                // If there is a selected item, that is the best one, so it has to be added to the
                // clipper, or it will be skipped.
                if let (Some(i_sel), true) = (self.selected, self.scroll_dirty) {
                    if let Some(idx) = self.visible_entries.iter().position(|i| *i == i_sel) {
                        clipper.add_included_range(idx..idx + 1);
                    }
                }
                clipper.with(|i| {
                    let i_entry = self.visible_entries[i];
                    let entry = &self.entries[i_entry];

                    ui.table_next_row(imgui::TableRowFlags::None, 0.0);

                    // File type
                    ui.table_set_column_index(0);
                    let icon_rr = match entry.kind {
                        FileEntryKind::Parent => Some(atlas.parent_rr),
                        FileEntryKind::Directory => Some(atlas.folder_rr),
                        FileEntryKind::File => Some(atlas.file_rr),
                        FileEntryKind::Root => Some(atlas.mypc_rr),
                    };
                    if let Some(rr) = icon_rr {
                        let avail = ui.get_content_region_avail();
                        let scale = ui.get_font_size() / 16.0;
                        let img_w = ui.font_atlas().get_custom_rect(rr).Width as f32;
                        ui.set_cursor_pos_x(
                            ui.get_cursor_pos_x() + (avail.x - scale * img_w) / 2.0,
                        );
                        ui.image_with_custom_rect_config(rr, scale).build();
                    }

                    // File name
                    ui.table_set_column_index(1);
                    let is_selected = Some(i_entry) == self.selected;
                    if ui
                        .selectable_config(entry.name.to_string_lossy().into())
                        .flags(
                            imgui::SelectableFlags::SpanAllColumns
                                | imgui::SelectableFlags::AllowOverlap
                                | imgui::SelectableFlags::AllowDoubleClick,
                        )
                        .selected(is_selected)
                        .build()
                    {
                        // Change the selected file
                        self.selected = Some(i_entry);
                        // Copy the selected name to `file_name`. Only regular files, no
                        // directories.
                        if entry.kind == FileEntryKind::File {
                            self.file_name = entry.name.clone();
                        }
                        // If double click, confirm the widget.
                        if ui.is_mouse_double_clicked(easy_imgui::MouseButton::Left) {
                            match entry.kind {
                                FileEntryKind::Parent => {
                                    next_path = self.path.parent().map(|p| p.to_owned());
                                }
                                FileEntryKind::Directory | FileEntryKind::Root => {
                                    next_path = Some(self.path.join(&entry.name));
                                }
                                FileEntryKind::File => {
                                    output = Output::Ok;
                                }
                            }
                        }
                    }

                    if is_selected && self.scroll_dirty {
                        self.scroll_dirty = false;
                        ui.set_scroll_here_y(0.5);
                    }

                    // File size
                    ui.table_set_column_index(2);
                    if let Some(size) = entry.size {
                        let text = format!("{}", ByteSize(size));
                        ui.text(&text);
                    }

                    // File modification time
                    ui.table_set_column_index(3);
                    if let Some(modified) = entry.modified {
                        let tm = time::OffsetDateTime::from(modified);
                        let s = tm
                            .format(format_description!(
                                "[year]-[month]-[day] [hour]:[minute]:[second]"
                            ))
                            .unwrap_or_default();
                        ui.text(&s);
                    }
                });
                if self.scroll_dirty {
                    self.scroll_dirty = false;
                    ui.set_scroll_y(0.0);
                }
            });
        if preview_width > 0.0 {
            ui.same_line();
            ui.child_config(lbl("preview"))
                .size(imgui::Vector2::new(0.0, -reserve))
                .with(|| preview.do_ui(ui, self));
        }

        ui.text(&tr!("File name"));
        ui.same_line();

        if self.filters.is_empty() {
            ui.set_next_item_width(-f32::EPSILON);
        } else {
            let filter_width = style.ItemSpacing.x
                + ui.calc_text_size(&self.filters[self.active_filter_idx].text)
                    .x
                + ui.get_frame_height()
                + style.ItemInnerSpacing.x;
            // Reasonable minimum default width?
            let filter_width = filter_width.max(ui.get_font_size() * 10.0);
            ui.set_next_item_width(-filter_width);
        }

        if ui.is_window_appearing() {
            ui.set_keyboard_focus_here(0);
        }
        ui.input_os_string_config(lbl_id(c"", c"input"), &mut self.file_name)
            .build();

        if !self.filters.is_empty() {
            ui.same_line();
            ui.set_next_item_width(-f32::EPSILON);
            if ui.combo(
                lbl_id(c"", c"Filter"),
                0..self.filters.len(),
                |i| &self.filters[i].text,
                &mut self.active_filter_idx,
            ) {
                self.visible_dirty = true;
            }
        }

        let font_sz = ui.get_font_size();
        let can_ok = !self.file_name.is_empty() && !self.path.as_os_str().is_empty();
        ui.with_disabled(!can_ok, || {
            if ui
                .button_config(lbl_id(tr!("OK"), "ok"))
                .size(imgui::Vector2::new(5.5 * font_sz, 0.0))
                .build()
                | ui.shortcut(imgui::Key::Enter)
                | ui.shortcut(imgui::Key::KeypadEnter)
            {
                output = Output::Ok;
            }
        });
        ui.same_line();
        if ui
            .button_config(lbl_id(tr!("Cancel"), "cancel"))
            .size(imgui::Vector2::new(5.5 * font_sz, 0.0))
            .build()
            | ui.shortcut(imgui::Key::Escape)
        {
            output = Output::Cancel;
        }
        if self.flags.contains(Flags::SHOW_READ_ONLY) {
            ui.same_line();
            let text = tr!("Read only");
            let check_width =
                ui.get_frame_height() + style.ItemInnerSpacing.x + ui.calc_text_size(&text).x;
            ui.set_cursor_pos_x(
                ui.get_cursor_pos_x() + ui.get_content_region_avail().x - check_width,
            );
            ui.checkbox(text.into(), &mut self.read_only);
        }

        if let Some(next_path) = next_path {
            let _ = self.set_path(next_path);
        }

        output
    }
    fn resort_entries(&mut self, specs: &[easy_imgui::TableColumnSortSpec]) {
        let sel = self.selected.map(|i| self.entries[i].name.clone());

        self.entries.sort_by(|a, b| {
            use std::cmp::Ordering;
            // .. is always the first, no matter the sort order
            use FileEntryKind::Parent;
            match (a.kind, b.kind) {
                (Parent, Parent) => return Ordering::Equal,
                (Parent, _) => return Ordering::Less,
                (_, Parent) => return Ordering::Greater,
                (_, _) => (),
            }
            for s in specs {
                let res = match s.index() {
                    0 => a.kind.cmp(&b.kind),
                    1 => a.name.cmp(&b.name),
                    2 => a.size.cmp(&b.size),
                    3 => a.modified.cmp(&b.modified),
                    _ => continue,
                };
                let res = match s.sort_direction() {
                    easy_imgui::SortDirection::Ascending => res,
                    easy_imgui::SortDirection::Descending => res.reverse(),
                    _ => continue,
                };
                if res.is_ne() {
                    return res;
                }
            }
            Ordering::Equal
        });
        // Restore the selected element
        self.selected = sel.and_then(|n| self.entries.iter().position(|e| e.name == n));
        // After a sort, recompute the filter because it stores the indices
        self.visible_dirty = true;
    }
    fn recompute_visible_entries(&mut self) {
        let search_term = self.search_term.to_lowercase();
        self.visible_entries.clear();
        self.visible_entries
            .extend(
                self.entries
                    .iter()
                    .enumerate()
                    .filter_map(|(i_entry, entry)| {
                        if !self.show_hidden && entry.hidden {
                            return None;
                        }
                        // Search term applies to both files and directories
                        if matches!(entry.kind, FileEntryKind::File | FileEntryKind::Directory)
                            && !search_term.is_empty()
                            && !entry
                                .name
                                .to_string_lossy()
                                .to_lowercase()
                                .contains(&search_term)
                        {
                            return None;
                        }
                        // Filters only apply to regular files
                        if entry.kind == FileEntryKind::File && !self.filters.is_empty() {
                            let f = &self.filters[self.active_filter_idx];
                            if !f.matches(&entry.name) {
                                return None;
                            }
                        }
                        Some(i_entry)
                    }),
            );
    }
}

/// Extra arguments for the `FileChooser::do_ui()` function.
pub struct UiParameters<'a, Preview> {
    atlas: &'a CustomAtlas,
    preview: Preview,
}

/// A trait to build the "preview" section of the UI.
pub trait PreviewBuilder<A> {
    /// The width reserved for the preview. Return 0.0 for no preview.
    fn width(&self) -> f32;
    /// Builds the UI for the preview.
    fn do_ui(&mut self, ui: &imgui::Ui<A>, chooser: &FileChooser);
}

/// A dummy implementation for `PreviewBuilder` that does nothing.
pub struct NoPreview;

impl<A> PreviewBuilder<A> for NoPreview {
    fn width(&self) -> f32 {
        0.0
    }
    fn do_ui(&mut self, _ui: &easy_imgui::Ui<A>, _chooser: &FileChooser) {}
}

impl<'a> UiParameters<'a, NoPreview> {
    /// Builds a `UiParameters` without preview.
    pub fn new(atlas: &'a CustomAtlas) -> Self {
        UiParameters {
            atlas,
            preview: NoPreview,
        }
    }
    /// Adds a preview object to this `UiParameters`.
    pub fn with_preview<A, P: PreviewBuilder<A>>(self, preview: P) -> UiParameters<'a, P> {
        UiParameters {
            atlas: self.atlas,
            preview,
        }
    }
}

/// Converts a `&CustomAtlas` into a UiParameters, with all other values to their default.
impl<'a> From<&'a CustomAtlas> for UiParameters<'a, NoPreview> {
    fn from(value: &'a CustomAtlas) -> Self {
        UiParameters::new(value)
    }
}

impl FileEntry {
    fn new(rd: &DirEntry) -> FileEntry {
        let name = rd.file_name();
        let (kind, size, modified, hidden);
        match rd.path().metadata() {
            Ok(meta) => {
                if meta.is_dir() {
                    kind = FileEntryKind::Directory;
                    size = None;
                } else {
                    kind = FileEntryKind::File;
                    size = Some(meta.len());
                }
                modified = meta.modified().ok();
                #[cfg(target_os = "windows")]
                {
                    use std::os::windows::fs::MetadataExt;
                    hidden = (meta.file_attributes() & 2) != 0; // FILE_ATTRIBUTE_HIDDEN
                }
            }
            Err(_) => {
                // Unknown kind, assume a file
                kind = FileEntryKind::File;
                size = None;
                modified = None;
                #[cfg(target_os = "windows")]
                {
                    hidden = false;
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            hidden = if matches!(kind, FileEntryKind::File | FileEntryKind::Directory) {
                name.as_encoded_bytes().first() == Some(&b'.')
            } else {
                false
            };
        }
        FileEntry {
            name,
            kind,
            size,
            modified,
            hidden,
        }
    }

    fn dot_dot() -> FileEntry {
        FileEntry {
            name: "..".into(),
            kind: FileEntryKind::Parent,
            size: None,
            modified: None,
            hidden: false,
        }
    }
}

macro_rules! image {
    ($name:ident = $file:literal) => {
        fn $name() -> &'static image::DynamicImage {
            static BYTES: &[u8] = include_bytes!($file);
            static IMG: std::sync::OnceLock<image::DynamicImage> = std::sync::OnceLock::new();
            IMG.get_or_init(|| {
                image::load_from_memory_with_format(BYTES, image::ImageFormat::Png).unwrap()
            })
        }
    };
}

image! {file_img = "file.png"}
image! {folder_img = "folder.png"}
image! {parent_img = "parent.png"}
image! {hidden_img = "hidden.png"}
#[cfg(target_os = "windows")]
image! {mypc_img = "mypc.png"}

/// Custom atlas for the FileChooser widget.
///
/// In order to get proper icons, you should build one of these when rebuilding your easy-imgui
/// atlas.
#[derive(Default, Copy, Clone)]
pub struct CustomAtlas {
    file_rr: CustomRectIndex,
    folder_rr: CustomRectIndex,
    parent_rr: CustomRectIndex,
    hidden_rr: CustomRectIndex,
    mypc_rr: CustomRectIndex,
}

/// Rebuild the custom atlas.
///
/// Call this on your `build_custom_atlas` impl and keep the output. You will need it to call
/// `do_ui`.
pub fn build_custom_atlas<A>(atlas: &mut easy_imgui::FontAtlasMut<A>) -> CustomAtlas {
    use image::GenericImage;

    let mut do_rr = move |img: &'static DynamicImage| {
        atlas.add_custom_rect_regular([img.width(), img.height()], {
            move |_, pixels| pixels.copy_from(img, 0, 0).unwrap()
        })
    };
    let file_rr = do_rr(file_img());
    let folder_rr = do_rr(folder_img());
    let parent_rr = do_rr(parent_img());
    let hidden_rr = do_rr(hidden_img());

    // This icon is only used on Windows
    #[cfg(target_os = "windows")]
    let mypc_rr = do_rr(mypc_img());
    #[cfg(not(target_os = "windows"))]
    let mypc_rr = Default::default();

    CustomAtlas {
        file_rr,
        folder_rr,
        parent_rr,
        hidden_rr,
        mypc_rr,
    }
}
