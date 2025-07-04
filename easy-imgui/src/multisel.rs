use super::*;

impl<A> Ui<A> {
    pub fn set_next_item_selection_user_data(&self, i: usize) {
        unsafe {
            ImGui_SetNextItemSelectionUserData(i as ImGuiSelectionUserData);
        }
    }
    pub fn is_item_toggled_selection(&self) -> bool {
        unsafe { ImGui_IsItemToggledSelection() }
    }
    /// Wraps the call to `f` between `BeginMultiselect` and `EndMultiselect`.
    ///
    /// The `storage` argument defines how the selection status will be stored.
    pub fn with_multi_select<R, Storage: MultiSelectStorage>(
        &self,
        flags: MultiSelectFlags,
        items_count: Option<usize>,
        mut storage: Storage,
        f: impl FnOnce(&mut Storage, &mut MultiSelect) -> R,
    ) -> R {
        let selection_size = storage
            .size()
            .and_then(|x| i32::try_from(x).ok())
            .unwrap_or(-1);
        let items_count = items_count
            .and_then(|x| i32::try_from(x).ok())
            .unwrap_or(-1);

        let ms = unsafe { ImGui_BeginMultiSelect(flags.bits(), selection_size, items_count) };
        let mut ms = MultiSelect(ms);
        storage.apply_requests(&mut ms);

        let res = f(&mut storage, &mut ms);

        let ms = unsafe { ImGui_EndMultiSelect() };
        let mut ms = MultiSelect(ms);
        storage.apply_requests(&mut ms);

        res
    }
}

pub struct MultiSelect(*mut ImGuiMultiSelectIO);

impl MultiSelect {
    pub fn range_src_item(&self) -> usize {
        unsafe { (*self.0).RangeSrcItem as usize }
    }
    pub fn nav_id_item(&self) -> usize {
        unsafe { (*self.0).NavIdItem as usize }
    }
    pub fn nav_id_selected(&self) -> bool {
        unsafe { (*self.0).NavIdSelected }
    }
    pub fn items_count(&self) -> usize {
        unsafe { (*self.0).ItemsCount as usize }
    }
    pub fn set_range_src_reset(&mut self) {
        unsafe {
            (*self.0).RangeSrcReset = true;
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = SelectionRequest<'_>> {
        unsafe { (*self.0).Requests.iter().map(SelectionRequest) }
    }
}

pub struct SelectionRequest<'a>(&'a ImGuiSelectionRequest);

impl SelectionRequest<'_> {
    pub fn request_type(&self) -> SelectionRequestType {
        SelectionRequestType::from_bits(self.0.Type).unwrap()
    }
    pub fn selected(&self) -> bool {
        self.0.Selected
    }
    pub fn range_direction(&self) -> i8 {
        self.0.RangeDirection
    }
    pub fn range_first_item(&self) -> usize {
        self.0.RangeFirstItem as usize
    }
    pub fn range_last_item(&self) -> usize {
        self.0.RangeLastItem as usize
    }
}

/// How the multi-select data will be stored.
///
/// There is a blank implementation for mutable references
/// to a type that implements this trait, so that you can use
/// a storage without consuming it. Or you can pass a temporary
/// adaptor without getting a reference.
pub trait MultiSelectStorage {
    fn size(&self) -> Option<usize>;
    fn apply_requests(&mut self, ms: &mut MultiSelect);
}

/// Use the storage without consuming it.
impl<T: MultiSelectStorage> MultiSelectStorage for &mut T {
    fn size(&self) -> Option<usize> {
        T::size(self)
    }
    fn apply_requests(&mut self, ms: &mut MultiSelect) {
        T::apply_requests(self, ms)
    }
}

////////////////////////

type BoxFnIdxToID<'a> = Box<dyn FnMut(usize) -> ImGuiID + 'a>;

extern "C" fn default_adapter_index_to_storage_id(
    _: *mut ImGuiSelectionBasicStorage,
    idx: i32,
) -> ImGuiID {
    idx as ImGuiID
}

extern "C" fn adapter_index_to_storage_id(
    this: *mut ImGuiSelectionBasicStorage,
    idx: i32,
) -> ImGuiID {
    unsafe {
        let f = (*this).UserData as *mut BoxFnIdxToID;
        if f.is_null() { 0 } else { (*f)(idx as usize) }
    }
}

/// Simple multi-selection storage facility.
///
/// By default it stores the indices in a collection.
/// Use the `with_callback_id()` function to do something different.
pub struct SelectionBasicStorage {
    inner: ImGuiSelectionBasicStorage,
}

impl SelectionBasicStorage {
    /// Creates a new selection storage.
    ///
    /// By default the index is the ID.
    pub fn new() -> Self {
        let mut inner = unsafe { ImGuiSelectionBasicStorage::new() };
        inner.AdapterIndexToStorageId = Some(default_adapter_index_to_storage_id);
        SelectionBasicStorage { inner }
    }
    /// Decorates this storage with a map function.
    ///
    /// The callback maps an index into an ID. This allows to re-sort
    /// the list without messing with the selection.
    pub fn with_callback_id<'a>(
        &'a mut self,
        f: impl FnMut(usize) -> ImGuiID + 'a,
    ) -> SelectionBasicStorageWithCallback<'a> {
        let f: BoxFnIdxToID<'a> = Box::new(f);
        let f = Box::new(f);
        self.inner.AdapterIndexToStorageId = Some(adapter_index_to_storage_id);
        SelectionBasicStorageWithCallback {
            inner: self,
            boxed_f: f,
        }
    }
    pub fn set_preserve_order(&mut self, preserve_order: bool) {
        self.inner.PreserveOrder = preserve_order;
    }
    pub fn contains(&self, id: ImGuiID) -> bool {
        unsafe { self.inner.Contains(id) }
    }
    pub fn clear(&mut self) {
        unsafe {
            self.inner.Clear();
        }
    }
    pub fn set_item_selected(&mut self, id: ImGuiID, selected: bool) {
        unsafe {
            self.inner.SetItemSelected(id, selected);
        }
    }
    pub fn swap(&mut self, other: &mut SelectionBasicStorage) {
        unsafe {
            self.inner.Swap(&mut other.inner);
        }
    }
    pub fn iter(&self) -> SelectionBasicStorageIter<'_> {
        SelectionBasicStorageIter {
            inner: self,
            ptr: std::ptr::null_mut(),
        }
    }
}

impl<'a> IntoIterator for &'a SelectionBasicStorage {
    type Item = ImGuiID;
    type IntoIter = SelectionBasicStorageIter<'a>;
    fn into_iter(self) -> SelectionBasicStorageIter<'a> {
        self.iter()
    }
}

pub struct SelectionBasicStorageIter<'a> {
    inner: &'a SelectionBasicStorage,
    ptr: *mut c_void,
}

impl Iterator for SelectionBasicStorageIter<'_> {
    type Item = ImGuiID;
    fn next(&mut self) -> Option<ImGuiID> {
        unsafe {
            let mut id = 0;
            // GetNextSelectedItem() is not const in C++, but it should be, I think so we can cast
            // the self argument into a mut pointer. It should be safe, probably.
            let this = &self.inner.inner as *const _ as *mut _;
            if ImGuiSelectionBasicStorage_GetNextSelectedItem(this, &mut self.ptr, &mut id) {
                Some(id)
            } else {
                None
            }
        }
    }
}

impl Default for SelectionBasicStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiSelectStorage for SelectionBasicStorage {
    fn size(&self) -> Option<usize> {
        Some(self.inner.Size as usize)
    }
    fn apply_requests(&mut self, ms: &mut MultiSelect) {
        unsafe {
            self.inner.ApplyRequests(ms.0);
        }
    }
}

pub struct SelectionBasicStorageWithCallback<'a> {
    inner: &'a mut SelectionBasicStorage,
    boxed_f: BoxFnIdxToID<'a>,
}

impl SelectionBasicStorageWithCallback<'_> {
    /// Gets the inner container.
    pub fn inner(&mut self) -> &mut SelectionBasicStorage {
        self.inner
    }
    /// Returns `self.inner().contains(id)`.
    ///
    /// Provided just for convenience.
    pub fn contains(&self, id: ImGuiID) -> bool {
        self.inner.contains(id)
    }
}

impl Drop for SelectionBasicStorageWithCallback<'_> {
    fn drop(&mut self) {
        // Restore the values before creating self.
        self.inner.inner.UserData = std::ptr::null_mut();
        self.inner.inner.AdapterIndexToStorageId = Some(default_adapter_index_to_storage_id);
    }
}

impl<'a> MultiSelectStorage for SelectionBasicStorageWithCallback<'a> {
    fn size(&self) -> Option<usize> {
        self.inner.size()
    }
    fn apply_requests(&mut self, ms: &mut MultiSelect) {
        self.inner.inner.UserData = &mut self.boxed_f as *mut BoxFnIdxToID<'a> as *mut c_void;
        self.inner.apply_requests(ms);
    }
}

/////////////////////

type BoxFnExtSetter<'f, Storage> = Box<dyn FnMut(&mut Storage, usize, bool) + 'f>;
type StorageAndSet<'f, Storage> = (Storage, BoxFnExtSetter<'f, Storage>);

/// MultiSelectStorage that forwards the selection data to another place.
///
/// `Storage` is the inner storage type, usually a mutable reference, but not
/// necessarily.
/// `'f` is the lifetime of the setter function, usually `'static`.
pub struct SelectionExternalStorage<'f, Storage> {
    inner: ImGuiSelectionExternalStorage,
    #[allow(clippy::type_complexity)]
    selection_size: Box<dyn Fn(&Storage) -> Option<usize> + 'f>,
    storage: StorageAndSet<'f, Storage>,
}

extern "C" fn adapter_set_item_selected<Storage>(
    this: *mut ImGuiSelectionExternalStorage,
    idx: i32,
    selected: bool,
) {
    unsafe {
        let storage_and_set = (*this).UserData as *mut StorageAndSet<'_, Storage>;
        let (storage, setter) = &mut *storage_and_set;
        setter(storage, idx as usize, selected);
    }
}

impl<'f, Storage> SelectionExternalStorage<'f, Storage> {
    /// Creates a new `SelectionExternalStorage`.
    ///
    /// The intended usage is that you keep the `storage` in your code
    /// and when multi-select happens, you pass a mutable reference to it in `storage`,
    /// along with a setter function to set the selection status of an item.
    ///
    /// Additionally, pass a function to count the number of selected items. If you don't
    /// want to compute that, use `|_| None`.
    pub fn new(
        storage: Storage,
        selection_size: impl Fn(&Storage) -> Option<usize> + 'f,
        setter: impl FnMut(&mut Storage, usize, bool) + 'f,
    ) -> Self {
        let mut inner = unsafe { ImGuiSelectionExternalStorage::new() };
        let setter: BoxFnExtSetter<'f, Storage> = Box::new(setter);
        let storage = (storage, setter);
        inner.AdapterSetItemSelected = Some(adapter_set_item_selected::<Storage>);
        SelectionExternalStorage {
            inner,
            selection_size: Box::new(selection_size),
            storage,
        }
    }
    /// Gets a reference to the inner storage.
    pub fn storage(&mut self) -> &mut Storage {
        &mut self.storage.0
    }
    /// Unwraps the inner storage.
    pub fn into_storage(self) -> Storage {
        self.storage.0
    }
}

impl<'f, Storage> MultiSelectStorage for SelectionExternalStorage<'f, Storage> {
    fn size(&self) -> Option<usize> {
        (self.selection_size)(&self.storage.0)
    }
    fn apply_requests(&mut self, ms: &mut MultiSelect) {
        self.inner.UserData = &mut self.storage as *mut StorageAndSet<'f, Storage> as *mut c_void;
        unsafe {
            self.inner.ApplyRequests(ms.0);
        }
    }
}
