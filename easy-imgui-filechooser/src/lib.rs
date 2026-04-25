/*!
 * A FileChooser widget for [`easy-imgui`](../easy_imgui/index.html).
 *
 * This widget does not create a window or a popup. It is up to you to create it in a
 * proper place.
 */

use bytesize::ByteSize;
use easy_imgui::{self as imgui, CustomRectIndex, id, lbl, lbl_id};
pub use glob::{self, Pattern};
use image::DynamicImage;
use std::io::Result;
use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    fs::DirEntry,
    path::{Path, PathBuf},
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

/// Trait to customize the filesystem view.
pub trait DirEnum {
    /// The roots of the filesystem, if more than one.
    ///
    /// If not empty FileChooser will show a virtual directory will all available roots.
    /// It should return entries of type [`FileEntryKind::Root`].
    ///
    /// In Windows, the roots are the drive letters: C:, D:...
    /// In normal systems, just return an empty iterator, and it will use the
    /// regular `/` as root.
    fn roots(&self) -> impl Iterator<Item = FileEntry>;
    /// Read the entries of a directory.
    fn read_dir<'s>(
        &'s self,
        path: &Path,
    ) -> std::io::Result<impl Iterator<Item = FileEntry> + use<'s, Self>>;
    /// Converts the given path to an absolute one.
    ///
    /// This is used to enumeate the parent directories if you start with a relative path.
    fn absolute(&self, path: &Path) -> std::io::Result<PathBuf>;
}

/// `DirEnum` is not object safe, to be able to use a type-erased dyn object, use this instead.
///
/// You can convert one into the other using `box_dir_enum`.
pub trait DynDirEnum {
    fn dyn_roots(&self) -> Box<dyn Iterator<Item = FileEntry> + '_>;
    fn dyn_read_dir<'s>(
        &'s self,
        path: &Path,
    ) -> std::io::Result<Box<dyn Iterator<Item = FileEntry> + 's>>;
    fn dyn_absolute(&self, path: &Path) -> std::io::Result<PathBuf>;
}

impl<T> DirEnum for T
where
    T: AsRef<dyn DynDirEnum>,
{
    fn roots(&self) -> impl Iterator<Item = FileEntry> {
        self.as_ref().dyn_roots()
    }
    fn read_dir<'s>(
        &'s self,
        path: &Path,
    ) -> std::io::Result<impl Iterator<Item = FileEntry> + use<'s, T>> {
        self.as_ref().dyn_read_dir(path)
    }
    fn absolute(&self, path: &Path) -> std::io::Result<PathBuf> {
        self.as_ref().dyn_absolute(path)
    }
}

impl<T: DirEnum> DynDirEnum for T {
    fn dyn_roots(&self) -> Box<dyn Iterator<Item = FileEntry> + '_> {
        Box::new(self.roots())
    }
    fn dyn_read_dir<'s>(
        &'s self,
        path: &Path,
    ) -> std::io::Result<Box<dyn Iterator<Item = FileEntry> + 's>> {
        Ok(Box::new(self.read_dir(path)?))
    }
    fn dyn_absolute(&self, path: &Path) -> std::io::Result<PathBuf> {
        self.absolute(path)
    }
}

/// Boxes a type that implements [`DirEnum`], erasing its type.
///
/// The returning type also implements `DirEnum`.
pub fn box_dir_enum(t: impl DirEnum + 'static) -> Box<dyn DynDirEnum> {
    Box::new(t)
}

#[cfg(feature = "zip")]
mod zipdirenum;
#[cfg(feature = "zip")]
pub use zipdirenum::{FileSystemDirEnumWithZip, ZipAnalyzeResult, ZipDirEnum};

/// The default implementation of [`DirEnum`].
///
/// Enumerates regular entries in the filesystem.
#[derive(Default)]
pub struct FileSystemDirEnum;

impl DirEnum for FileSystemDirEnum {
    #[cfg(not(target_os = "windows"))]
    fn roots(&self) -> impl Iterator<Item = FileEntry> {
        std::iter::empty()
    }
    #[cfg(target_os = "windows")]
    fn roots(&self) -> impl Iterator<Item = FileEntry> {
        struct Roots {
            drives: u32,
            letter: u8,
        }
        impl Iterator for Roots {
            type Item = FileEntry;

            fn next(&mut self) -> Option<FileEntry> {
                while self.letter < 26 {
                    // A .. Z
                    let bit = 1 << self.letter;
                    let drive = char::from(b'A' + self.letter);
                    self.letter += 1;
                    if (self.drives & bit) != 0 {
                        return Some(FileEntry {
                            name: format!("{}:\\", drive).into(),
                            kind: FileEntryKind::Root,
                            size: None,
                            modified: None,
                            hidden: false,
                        });
                    }
                }
                None
            }
        }
        let drives = unsafe { windows::Win32::Storage::FileSystem::GetLogicalDrives() };
        Roots { drives, letter: 0 }
    }

    fn read_dir<'s>(
        &'s self,
        path: &Path,
    ) -> std::io::Result<impl Iterator<Item = FileEntry> + use<'s>> {
        let rd = path.read_dir()?;
        Ok(rd.filter_map(|e| {
            let e = e.ok()?;
            Some(FileEntry::new(&e))
        }))
    }
    fn absolute(&self, path: &Path) -> std::io::Result<PathBuf> {
        std::path::absolute(path)
    }
}

/// The default `FileChooser` type.
pub type FileChooser = FileChooserD<FileSystemDirEnum>;

/// Main widget to create a file chooser.
///
/// Create one of these when the widget is opened, and then call `do_ui` for each frame.
pub struct FileChooserD<D: DirEnum> {
    dir_enum: D,
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

impl<D: DirEnum> std::fmt::Debug for FileChooserD<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileChooser")
            .field("path", &self.path())
            .field("file_name", &self.file_name())
            .field("active_filter", &self.active_filter())
            .field("read_only", &self.read_only())
            .finish()
    }
}

/// Kind of entry in the filesystem.
///
/// It affects the icon and behavior.
#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileEntryKind {
    /// Used for the `..` present in most directories.
    Parent,
    /// It is a directory, not a file.
    Directory,
    /// It is a regular file.
    File,
    /// Entry from the [`DirEnum::roots`] function.
    Root,
}

/// An entry in a directory.
#[derive(Debug)]
pub struct FileEntry {
    /// The name of the entry.
    pub name: OsString,
    /// The type of entry.
    pub kind: FileEntryKind,
    /// The size of the entry in bytes, if known.
    pub size: Option<u64>,
    /// The last modified time of the entry, if known.
    pub modified: Option<time::OffsetDateTime>,
    /// Whether this entry is considered hidden.
    pub hidden: bool,
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

bitflags::bitflags! {
    /// Flags that modify the default behavior of the `FileChooser`.
    pub struct Flags: u32 {
        /// Shows the "Read only" check.
        const SHOW_READ_ONLY = 1;
        /// Doesn't allow to select a non-existing file.
        const MUST_EXIST = 2;
    }
}

impl<D: DirEnum + Default> Default for FileChooserD<D> {
    fn default() -> Self {
        FileChooserD::with_dir_enum(D::default())
    }
}

impl<D: DirEnum + Default> FileChooserD<D> {
    /// Creates a `FileChooser` dialog with default options.
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ApplicablePathRes {
    Directory,
    ExistingFile,
    NewEntry,
    Forbidden,
}

impl<D: DirEnum> FileChooserD<D> {
    /// Creates a new `FileChooser` with the given `DirEnum`.
    pub fn with_dir_enum(dir_enum: D) -> FileChooserD<D> {
        FileChooserD {
            dir_enum,
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
        let path = self.dir_enum.absolute(path.as_ref())?;
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

        match path.parent() {
            Some(t) if !t.as_os_str().is_empty() => {
                add_entry(FileEntry::dot_dot());
            }
            _ => {}
        }

        for fe in self.dir_enum.read_dir(&path)? {
            add_entry(fe);
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

    /// Gets the selected entry, if any.
    pub fn selected_entry(&self) -> Option<&FileEntry> {
        let i_sel = self.selected?;
        Some(&self.entries[i_sel])
    }

    /// Gets the current file name.
    ///
    /// To get the final selection it is usually better to use the `full_path`.
    /// This is more useful for interactive things, such as previews.
    pub fn file_name(&self) -> &OsStr {
        // If the selected item maches the typed name (not counting lossy bits), then
        // use the original entry.name, that will better represent the original OsString,
        // and likely the user intent.
        // This is important only if the file has non-UTF-8 sequences.
        if let Some(entry) = self.selected_entry()
            && entry.kind == FileEntryKind::File
            && self.file_name == *entry.name.to_string_lossy()
        {
            return &entry.name;
        }
        &self.file_name
    }

    /// Gets the current active filter, if any.
    ///
    /// It is None only if no filters have been added.
    pub fn active_filter(&self) -> Option<FilterId> {
        if self.filters.is_empty() {
            None
        } else {
            Some(self.filters[self.active_filter_idx].id)
        }
    }
    /// Sets the current active filter.
    ///
    /// Adding filters can change the active filter, so for best results
    /// do this after all filter have been added.
    pub fn set_active_filter(&mut self, filter_id: FilterId) {
        if let Some(p) = self.filters.iter().position(|f| f.id == filter_id) {
            self.active_filter_idx = p;
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
        let (mut res, exists) = self.applicable_path();
        if let (None, Some(new_ext), ApplicablePathRes::NewEntry) =
            (res.extension(), default_extension, exists)
        {
            res.set_extension(new_ext);
        }
        res
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
        Preview: PreviewBuilder<A, D>,
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
                let mut roots = self.dir_enum.roots();
                let r0 = roots.next();
                if let Some(r0) = r0 {
                    let scale = ui.get_font_size() / 16.0;
                    if ui
                        .image_button_with_custom_rect_config(id("::super"), atlas.mypc_rr, scale)
                        .build()
                    {
                        // TODO: partial duplicate of `set_path()`
                        self.entries.clear();
                        self.entries.push(r0);
                        self.entries.extend(roots);
                        self.path = PathBuf::default();
                        self.selected = None;
                        self.sort_dirty = true;
                        self.visible_dirty = true;
                        self.scroll_dirty = true;
                        self.popup_dirs.clear();
                        self.search_term.clear();
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
                            if let Ok(subdir) = self.dir_enum.read_dir(parent) {
                                for d in subdir {
                                    if d.kind != FileEntryKind::Directory {
                                        continue;
                                    }
                                    if d.hidden && !self.show_hidden {
                                        continue;
                                    }

                                    let full_d = parent.join(d.name);
                                    let sel = full_d == my_path;
                                    popup_dirs.push((full_d, sel));
                                }
                            }
                        } else {
                            // TODO: filter out when there is only one?
                            for root in self.dir_enum.roots() {
                                let root = PathBuf::from(root.name);
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
                if let (Some(i_sel), true) = (self.selected, self.scroll_dirty)
                    && let Some(idx) = self.visible_entries.iter().position(|i| *i == i_sel)
                {
                    clipper.add_included_range(idx..idx + 1);
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
                        let img_w = ui.get_custom_rect(rr).unwrap().rect.w as f32;
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
                        // Copy the selected name to `file_name`
                        self.file_name = entry.name.clone();
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
                        let s = modified
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
        let press_enter = ui
            .input_os_string_config(lbl_id(c"", c"input"), &mut self.file_name)
            .flags(imgui::InputTextFlags::EnterReturnsTrue)
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

        let (maybe_next_path, exists) = self.applicable_path();
        let can_ok = exists != ApplicablePathRes::Forbidden;
        ui.with_disabled(!can_ok, || {
            if ui
                .button_config(lbl_id(tr!("OK"), "ok"))
                .size(imgui::Vector2::new(5.5 * font_sz, 0.0))
                .build()
                | ui.shortcut(imgui::Key::Enter)
                | ui.shortcut(imgui::Key::KeypadEnter)
                | (can_ok && press_enter)
            {
                // Maybe activate something...
                match exists {
                    // It is a dir: navigate
                    ApplicablePathRes::Directory => next_path = Some(maybe_next_path),
                    // It is a file: accept
                    ApplicablePathRes::ExistingFile => output = Output::Ok,
                    // New file: accept
                    ApplicablePathRes::NewEntry => output = Output::Ok,
                    // Invalid: ignore
                    ApplicablePathRes::Forbidden => (),
                }
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
            // Changing the directory deletes the typed name, because it is usually the directory itself, no longer useful.
            self.set_file_name("");
        }

        output
    }

    fn applicable_path(&self) -> (PathBuf, ApplicablePathRes) {
        // Path::normalize_lexically would be handy here.
        let mut file_name = self.file_name();

        if file_name == "" {
            (PathBuf::new(), ApplicablePathRes::Forbidden)
        } else if file_name == "." {
            (self.path.to_path_buf(), ApplicablePathRes::Directory)
        } else if file_name == ".." {
            let p = self
                .path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or(self.path.clone());
            (p, ApplicablePathRes::Directory)
        } else {
            let exists = self
                .visible_entries
                .iter()
                .map(|&i| &self.entries[i])
                .find_map(|e| {
                    if e.name == file_name {
                        match e.kind {
                            FileEntryKind::File => {
                                file_name = &e.name;
                                Some(ApplicablePathRes::ExistingFile)
                            }
                            _ => Some(ApplicablePathRes::Directory),
                        }
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    // If there is no entry with the exact name, look for one without the extension
                    if Path::new(&file_name).extension().is_some() {
                        return None;
                    }
                    let candidate = self
                        .visible_entries
                        .iter()
                        .map(|&i| &self.entries[i])
                        .find_map(|e| {
                            if e.kind != FileEntryKind::File {
                                return None;
                            }
                            let p = Path::new(&e.name);
                            if p.file_stem() != Some(file_name) {
                                return None;
                            }
                            Some(&e.name)
                        })?;
                    file_name = candidate;
                    Some(ApplicablePathRes::ExistingFile)
                })
                .unwrap_or_else(|| {
                    if self.flags.contains(Flags::MUST_EXIST) {
                        ApplicablePathRes::Forbidden
                    } else {
                        ApplicablePathRes::NewEntry
                    }
                });
            (self.path.join(file_name), exists)
        }
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
pub trait PreviewBuilder<A, D: DirEnum = FileSystemDirEnum> {
    /// The width reserved for the preview. Return 0.0 for no preview.
    fn width(&self) -> f32;
    /// Builds the UI for the preview.
    fn do_ui(&mut self, ui: &imgui::Ui<A>, chooser: &FileChooserD<D>);
}

/// A dummy implementation for `PreviewBuilder` that does nothing.
pub struct NoPreview;

impl<A, D: DirEnum> PreviewBuilder<A, D> for NoPreview {
    fn width(&self) -> f32 {
        0.0
    }
    fn do_ui(&mut self, _ui: &easy_imgui::Ui<A>, _chooser: &FileChooserD<D>) {}
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
    pub fn with_preview<A, D: DirEnum, P: PreviewBuilder<A, D>>(
        self,
        preview: P,
    ) -> UiParameters<'a, P> {
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
                modified = meta.modified().ok().map(time::OffsetDateTime::from);
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
/// Call this on your initialization code and keep the output. You will need it to call
/// `do_ui`.
pub fn build_custom_atlas(atlas: &mut easy_imgui::FontAtlas) -> CustomAtlas {
    use image::GenericImage;

    let mut do_rr = |img: &'static DynamicImage| {
        atlas.add_custom_rect([img.width(), img.height()], {
            |pixels| pixels.copy_from(img, 0, 0).unwrap()
        })
    };
    let file_rr = do_rr(file_img());
    let folder_rr = do_rr(folder_img());
    let parent_rr = do_rr(parent_img());
    let hidden_rr = do_rr(hidden_img());
    let mypc_rr = do_rr(mypc_img());

    CustomAtlas {
        file_rr,
        folder_rr,
        parent_rr,
        hidden_rr,
        mypc_rr,
    }
}
