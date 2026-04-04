use std::cell::RefCell;
use std::collections::BTreeMap;

use super::*;

#[derive(Default, Debug)]
struct ZipDir {
    entries: BTreeMap<OsString, ZipEntry>,
}

#[derive(Debug)]
enum ZipEntry {
    Directory(ZipDir),
    File {
        size: u64,
        modified: Option<zip::DateTime>,
    },
}

pub struct ZipDirEnum {
    root: ZipEntry,
}

impl ZipDirEnum {
    pub fn new<R: std::io::Read + std::io::Seek>(zip: &mut zip::ZipArchive<R>) -> ZipDirEnum {
        let mut root = ZipEntry::Directory(ZipDir::default());
        for i in 0..zip.len() {
            let Ok(zentry) = zip.by_index(i) else {
                continue;
            };
            // Directory entries in a ZIP file? is that even possible?
            if zentry.is_dir() {
                continue;
            }
            let path = PathBuf::from(zentry.name());
            let mut cur = &mut root;

            for piece in &path {
                if !matches!(cur, ZipEntry::Directory(_)) {
                    *cur = ZipEntry::Directory(ZipDir::default());
                }
                let ZipEntry::Directory(ZipDir { entries }) = cur else {
                    unreachable!()
                };

                let entry = entries.entry(piece.to_owned());
                cur = entry.or_insert_with(|| ZipEntry::Directory(ZipDir::default()));
            }

            let size = zentry.size();
            let modified = zentry.last_modified();
            // TODO ze.extra_data_fields()
            *cur = ZipEntry::File { size, modified };
        }

        ZipDirEnum { root }
    }

    fn get(&self, path: &Path) -> Option<&ZipEntry> {
        let mut cur = &self.root;
        for piece in path {
            if piece == "/" || piece == "." {
                continue;
            }
            let ZipEntry::Directory(dir) = cur else {
                return None;
            };
            cur = dir.entries.get(piece)?;
        }
        Some(cur)
    }
}

impl DirEnum for ZipDirEnum {
    fn roots(&self) -> impl Iterator<Item = FileEntry> {
        std::iter::empty()
    }

    fn read_dir<'s>(
        &'s self,
        path: &Path,
    ) -> std::io::Result<impl Iterator<Item = FileEntry> + use<'s>> {
        let entry = self.get(path).ok_or(std::io::ErrorKind::NotFound)?;
        let ZipEntry::Directory(dir) = entry else {
            return Err(std::io::ErrorKind::NotADirectory.into());
        };
        Ok(dir.entries.iter().map(|(k, v)| {
            let name = k.to_owned();
            match v {
                ZipEntry::Directory(_) => FileEntry {
                    name,
                    kind: FileEntryKind::Directory,
                    size: None,
                    modified: None,
                    hidden: false,
                },
                ZipEntry::File { size, modified } => {
                    let modified = modified.and_then(|m| {
                        let d = time::Date::from_calendar_date(
                            m.year().into(),
                            time::Month::try_from(m.month()).ok()?,
                            m.day(),
                        )
                        .ok()?;
                        let t = time::Time::from_hms(m.hour(), m.minute(), m.second()).ok()?;
                        Some(time::OffsetDateTime::new_utc(d, t))
                    });
                    FileEntry {
                        name,
                        kind: FileEntryKind::File,
                        size: Some(*size),
                        modified,
                        hidden: false,
                    }
                }
            }
        }))
    }

    fn absolute(&self, path: &Path) -> std::io::Result<PathBuf> {
        if path.is_relative() {
            Ok(PathBuf::from("/").join(path))
        } else {
            Ok(path.into())
        }
    }
}

#[derive(Default)]
pub struct FileSystemDirEnumWithZip {
    fs: FileSystemDirEnum,
    zip: RefCell<Option<(PathBuf, ZipDirEnum)>>,
}

/// Return type for `FileSystemDirEnumWithZip::analyze`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ZipAnalyzeResult<'a> {
    /// It is a normal entry in the filesystem, or maybe an unexisting or invalid entry.
    Regular(&'a Path),
    /// It is an entry in a zip file. Note that the zip file or the entry may not exist.
    Zip { zip_name: &'a Path, inner: &'a Path },
}

impl FileSystemDirEnumWithZip {
    pub fn new() -> FileSystemDirEnumWithZip {
        FileSystemDirEnumWithZip::default()
    }

    /// Analyzes the given `path` in the context of maybe opening a ZIP file.
    ///
    /// Returns `ZipAnalyzeResult::Regular` if the giving path is normal file in the filesystem.
    /// Returns `ZipAnalyzeResult::Zip` if the path is an entry inside a ZIP file.
    /// If the input path is inconclusive or erroneous, it will default to `Regular`, because the
    /// receiving code must already be prepared to get invalid paths.
    pub fn analyze<'a>(path: &'a Path) -> ZipAnalyzeResult<'a> {
        // If the path is real, it is `Regular`, no matter the name.
        if path.exists() {
            if !path.is_file() || path.extension().map(|x| x == "zip") != Some(true) {
                return ZipAnalyzeResult::Regular(path);
            }
        }
        // Check the parents to see if there is a real zip file instead of a directory.
        let mut parent = path;
        loop {
            if parent.is_dir() {
                return ZipAnalyzeResult::Regular(path);
            }
            if parent.is_file() {
                let inner = match path.strip_prefix(parent) {
                    Ok(p) => p,
                    // I don't think this can ever fail, but just in case
                    Err(_) => return ZipAnalyzeResult::Regular(path),
                };
                return ZipAnalyzeResult::Zip {
                    zip_name: parent,
                    inner,
                };
            }
            parent = match parent.parent() {
                Some(p) => p,
                None => return ZipAnalyzeResult::Regular(path),
            }
        }
    }
}

impl DirEnum for FileSystemDirEnumWithZip {
    fn roots(&self) -> impl Iterator<Item = FileEntry> {
        self.fs.roots()
    }

    fn read_dir<'s>(
        &'s self,
        path: &Path,
    ) -> std::io::Result<impl Iterator<Item = FileEntry> + use<'s>> {
        match Self::analyze(path) {
            ZipAnalyzeResult::Regular(path) => self.fs.read_dir(path).map(|it| {
                let it = it.map(|mut d| {
                    if d.kind == FileEntryKind::File {
                        let p = Path::new(&d.name);
                        if p.extension().map(|x| x == "zip") == Some(true) {
                            d.kind = FileEntryKind::Directory;
                        }
                    }
                    d
                });
                easy_imgui::Either::Left(it)
            }),
            ZipAnalyzeResult::Zip { zip_name, inner } => {
                let mut zip = self.zip.borrow_mut();
                // Check if the zip file has changed
                match &*zip {
                    Some((name, _)) if name == zip_name => {}
                    _ => {
                        let z = std::fs::File::open(zip_name)?;
                        let z = std::io::BufReader::new(z);
                        let mut z = zip::ZipArchive::new(z)?;
                        *zip = Some((zip_name.to_owned(), ZipDirEnum::new(&mut z)));
                    }
                }
                let dir = zip.as_ref().unwrap().1.read_dir(inner)?.collect::<Vec<_>>();
                Ok(easy_imgui::Either::Right(dir.into_iter()))
            }
        }
    }

    fn absolute(&self, path: &Path) -> std::io::Result<PathBuf> {
        std::path::absolute(path)
    }
}
