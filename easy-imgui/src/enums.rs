// These enums have the same name as their C++ equivalent, do not warn about it
#![allow(non_upper_case_globals)]

use easy_imgui_sys::*;

// In most API calls enums are passed as integers, but a few are true enums.
// But since the code to wrap the enums is created by a macro, we use this trait
// to do the necessary conversions.

use std::ffi::c_int;

trait BitEnumHelper {
    fn to_bits(self) -> c_int;
    fn from_bits(t: c_int) -> Self;
}

impl BitEnumHelper for c_int {
    #[inline]
    fn to_bits(self) -> c_int {
        self
    }
    #[inline]
    fn from_bits(t: c_int) -> Self {
        t
    }
}

macro_rules! impl_bit_enum_helper {
    ($native_name:ident) => {
        impl BitEnumHelper for $native_name {
            #[inline]
            fn to_bits(self) -> c_int {
                self.0 as _
            }
            #[inline]
            fn from_bits(t: c_int) -> Self {
                Self(t as _)
            }
        }
    };
}

macro_rules! imgui_enum_ex {
    ($(#[$attr:meta])* $vis:vis $name:ident : $native_name:ident : $native_name_api:ty { $( $(#[$inner:ident $($args:tt)*])* $field:ident = $value:ident),* $(,)? }) => {
        $(#[$attr])*
        #[repr(i32)]
        #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
        $vis enum $name {
            $(
                $(#[$inner $($args)*])*
                $field = $native_name::$value.0 as i32,
            )*
        }
        impl $name {
            pub fn bits(self) -> $native_name_api {
                <$native_name_api>::from_bits(self as c_int)
            }
            pub fn from_bits(bits: $native_name_api) -> Option<Self> {
                $(
                    $(#[$inner $($args)*])*
                    const $field: c_int = $native_name::$value.0 as i32;
                )*
                let r = match <$native_name_api>::to_bits(bits) {
                    $(
                        #[allow(unused_doc_comments)]
                        $(#[$inner $($args)*])*
                        $field => Self::$field,
                    )*
                    _ => return std::option::Option::None,
                };
                Some(r)
            }
        }
    };
}

macro_rules! imgui_enum {
    ($(#[$attr:meta])* $vis:vis $name:ident: $native_name:ident { $( $(#[$inner:ident $($args:tt)*])* $field:ident ),* $(,)? }) => {
        paste::paste! {
            imgui_enum_ex! {
                $(#[$attr])*
                $vis $name: $native_name: i32 {
                    $( $(#[$inner $($args)*])* $field = [<$native_name $field>],)*
                }
            }
        }
    };
}

// Just like imgui_enum but for native strong C++ enums
macro_rules! imgui_scoped_enum {
    ($(#[$attr:meta])* $vis:vis $name:ident: $native_name:ident { $( $(#[$inner:ident $($args:tt)*])* $field:ident ),* $(,)? }) => {
        impl_bit_enum_helper!{$native_name}
        paste::paste! {
            imgui_enum_ex! {
                $(#[$attr])*
                $vis $name: $native_name: $native_name {
                    $( $(#[$inner $($args)*])* $field = [<$native_name _ $field>],)*
                }
            }
        }
    };
}

macro_rules! imgui_flags_ex {
    ($(#[$attr:meta])* $vis:vis $name:ident: $native_name:ident { $( $(#[$inner:ident $($args:tt)*])* $field:ident = $($value:ident)::*),* $(,)? }) => {
        bitflags::bitflags! {
            $(#[$attr])*
            #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
            $vis struct $name : i32 {
                $(
                    $(#[$inner $($args)*])*
                    const $field = imgui_flags_ex! { @FIELD $native_name :: $($value)::* };
                )*
            }
        }
    };
    (@FIELD $native_name:ident :: $value:ident) => {
        $native_name::$value.0 as i32
    };
    (@FIELD $native_name:ident :: $alt_native_name:ident :: $value:ident) => {
        // ignore native_name, use alt_native_name instead
        $alt_native_name::$value.0 as i32
    };
}

macro_rules! imgui_flags {
    ($(#[$attr:meta])* $vis:vis $name:ident: $native_name:ident { $( $(#[$inner:ident $($args:tt)*])* $field:ident),* $(,)? }) => {
        paste::paste! {
            imgui_flags_ex! {
                $(#[$attr])*
                $vis $name: $native_name {
                    $( $(#[$inner $($args)*])* $field = [<$native_name $field>],)*
                }
            }
        }
    };
}

imgui_flags! {
    /// Dear ImGui (`ImDrawFlags`): Flags for `ImDrawList` functions
    pub DrawFlags: ImDrawFlags_ {
        /// Dear ImGui (`ImDrawFlags_None`): No flags
        None,
        /// Dear ImGui (`ImDrawFlags_Closed`): `PathStroke()`, `AddPolyline()`: specify that shape should be closed
        Closed,
        /// Dear ImGui (`ImDrawFlags_RoundCornersTopLeft`): `AddRect()`, `AddRectFilled()`, `PathRect()`: enable rounding top-left corner only
        RoundCornersTopLeft,
        /// Dear ImGui (`ImDrawFlags_RoundCornersTopRight`): `AddRect()`, `AddRectFilled()`, `PathRect()`: enable rounding top-right corner only
        RoundCornersTopRight,
        /// Dear ImGui (`ImDrawFlags_RoundCornersBottomLeft`): `AddRect()`, `AddRectFilled()`, `PathRect()`: enable rounding bottom-left corner only
        RoundCornersBottomLeft,
        /// Dear ImGui (`ImDrawFlags_RoundCornersBottomRight`): `AddRect()`, `AddRectFilled()`, `PathRect()`: enable rounding bottom-right corner only
        RoundCornersBottomRight,
        /// Dear ImGui (`ImDrawFlags_RoundCornersNone`): `AddRect()`, `AddRectFilled()`, `PathRect()`: disable rounding on all corners
        RoundCornersNone,
        /// Dear ImGui (`ImDrawFlags_RoundCornersTop`): `ImDrawFlags_RoundCornersTopLeft` | `ImDrawFlags_RoundCornersTopRight`
        RoundCornersTop,
        /// Dear ImGui (`ImDrawFlags_RoundCornersBottom`): `ImDrawFlags_RoundCornersBottomLeft` | `ImDrawFlags_RoundCornersBottomRight`
        RoundCornersBottom,
        /// Dear ImGui (`ImDrawFlags_RoundCornersLeft`): `ImDrawFlags_RoundCornersBottomLeft` | `ImDrawFlags_RoundCornersTopLeft`
        RoundCornersLeft,
        /// Dear ImGui (`ImDrawFlags_RoundCornersRight`): `ImDrawFlags_RoundCornersBottomRight` | `ImDrawFlags_RoundCornersTopRight`
        RoundCornersRight,
        /// Dear ImGui (`ImDrawFlags_RoundCornersAll`): All corners
        RoundCornersAll,
    }
}

imgui_enum! {
    /// Dear ImGui (`ImGuiCond`): Condition for many Set*() functions
    pub Cond: ImGuiCond_ {
        /// Dear ImGui (`ImGuiCond_Always`): No condition (always set the variable)
        Always,
        /// Dear ImGui (`ImGuiCond_Once`): Set the variable once per runtime session
        Once,
        /// Dear ImGui (`ImGuiCond_FirstUseEver`): Set the variable if the object/window has no persistently saved data
        FirstUseEver,
        /// Dear ImGui (`ImGuiCond_Appearing`): Set the variable if the object/window is appearing after being hidden/inactive
        Appearing,
    }
}

imgui_enum! {
    /// Dear ImGui (`ImGuiCol`): Color identifier for styling
    pub ColorId: ImGuiCol_ {
        /// Dear ImGui (`ImGuiCol_Text`): Text
        Text,
        /// Dear ImGui (`ImGuiCol_TextDisabled`): Text disabled
        TextDisabled,
        /// Dear ImGui (`ImGuiCol_WindowBg`): Background of normal windows
        WindowBg,
        /// Dear ImGui (`ImGuiCol_ChildBg`): Background of child windows
        ChildBg,
        /// Dear ImGui (`ImGuiCol_PopupBg`): Background of popups, menus, tooltips windows
        PopupBg,
        /// Dear ImGui (`ImGuiCol_Border`): Border
        Border,
        /// Dear ImGui (`ImGuiCol_BorderShadow`): Border shadow
        BorderShadow,
        /// Dear ImGui (`ImGuiCol_FrameBg`): Background of checkbox, radio button, plot, slider, text input
        FrameBg,
        /// Dear ImGui (`ImGuiCol_FrameBgHovered`): Frame background hovered
        FrameBgHovered,
        /// Dear ImGui (`ImGuiCol_FrameBgActive`): Frame background active
        FrameBgActive,
        /// Dear ImGui (`ImGuiCol_TitleBg`): Title bar
        TitleBg,
        /// Dear ImGui (`ImGuiCol_TitleBgActive`): Title bar when focused
        TitleBgActive,
        /// Dear ImGui (`ImGuiCol_TitleBgCollapsed`): Title bar when collapsed
        TitleBgCollapsed,
        /// Dear ImGui (`ImGuiCol_MenuBarBg`): Menu bar background
        MenuBarBg,
        /// Dear ImGui (`ImGuiCol_ScrollbarBg`): Scrollbar background
        ScrollbarBg,
        /// Dear ImGui (`ImGuiCol_ScrollbarGrab`): Scrollbar grab
        ScrollbarGrab,
        /// Dear ImGui (`ImGuiCol_ScrollbarGrabHovered`): Scrollbar grab hovered
        ScrollbarGrabHovered,
        /// Dear ImGui (`ImGuiCol_ScrollbarGrabActive`): Scrollbar grab active
        ScrollbarGrabActive,
        /// Dear ImGui (`ImGuiCol_CheckMark`): Checkbox tick and RadioButton circle
        CheckMark,
        /// Dear ImGui (`ImGuiCol_CheckboxSelectedBg`): Checkbox background when selected
        CheckboxSelectedBg,
        /// Dear ImGui (`ImGuiCol_SliderGrab`): Slider grab
        SliderGrab,
        /// Dear ImGui (`ImGuiCol_SliderGrabActive`): Slider grab active
        SliderGrabActive,
        /// Dear ImGui (`ImGuiCol_Button`): Button
        Button,
        /// Dear ImGui (`ImGuiCol_ButtonHovered`): Button hovered
        ButtonHovered,
        /// Dear ImGui (`ImGuiCol_ButtonActive`): Button active
        ButtonActive,
        /// Dear ImGui (`ImGuiCol_Header`): Header colors for CollapsingHeader, TreeNode, Selectable, MenuItem
        Header,
        /// Dear ImGui (`ImGuiCol_HeaderHovered`): Header hovered
        HeaderHovered,
        /// Dear ImGui (`ImGuiCol_HeaderActive`): Header active
        HeaderActive,
        /// Dear ImGui (`ImGuiCol_Separator`): Separator
        Separator,
        /// Dear ImGui (`ImGuiCol_SeparatorHovered`): Separator hovered
        SeparatorHovered,
        /// Dear ImGui (`ImGuiCol_SeparatorActive`): Separator active
        SeparatorActive,
        /// Dear ImGui (`ImGuiCol_ResizeGrip`): Resize grip
        ResizeGrip,
        /// Dear ImGui (`ImGuiCol_ResizeGripHovered`): Resize grip hovered
        ResizeGripHovered,
        /// Dear ImGui (`ImGuiCol_ResizeGripActive`): Resize grip active
        ResizeGripActive,
        /// Dear ImGui (`ImGuiCol_InputTextCursor`): InputText cursor/caret
        InputTextCursor,
        /// Dear ImGui (`ImGuiCol_TabHovered`): Tab background, when hovered
        TabHovered,
        /// Dear ImGui (`ImGuiCol_Tab`): Tab background, when tab-bar is focused & tab is unselected
        Tab,
        /// Dear ImGui (`ImGuiCol_TabSelected`): Tab background, when tab-bar is focused & tab is selected
        TabSelected,
        /// Dear ImGui (`ImGuiCol_TabSelectedOverline`): Tab horizontal overline, when tab-bar is focused & tab is selected
        TabSelectedOverline,
        /// Dear ImGui (`ImGuiCol_TabDimmed`): Tab background, when tab-bar is unfocused & tab is unselected
        TabDimmed,
        /// Dear ImGui (`ImGuiCol_TabDimmedSelected`): Tab background, when tab-bar is unfocused & tab is selected
        TabDimmedSelected,
        /// Dear ImGui (`ImGuiCol_TabDimmedSelectedOverline`): Tab horizontal overline, when tab-bar is unfocused & tab is selected
        TabDimmedSelectedOverline,
        /// Dear ImGui (`ImGuiCol_DockingPreview`): Preview overlay color when about to docking something
        DockingPreview,
        /// Dear ImGui (`ImGuiCol_DockingEmptyBg`): Background color for empty node
        DockingEmptyBg,
        /// Dear ImGui (`ImGuiCol_PlotLines`): Plot lines
        PlotLines,
        /// Dear ImGui (`ImGuiCol_PlotLinesHovered`): Plot lines hovered
        PlotLinesHovered,
        /// Dear ImGui (`ImGuiCol_PlotHistogram`): Plot histogram
        PlotHistogram,
        /// Dear ImGui (`ImGuiCol_PlotHistogramHovered`): Plot histogram hovered
        PlotHistogramHovered,
        /// Dear ImGui (`ImGuiCol_TableHeaderBg`): Table header background
        TableHeaderBg,
        /// Dear ImGui (`ImGuiCol_TableBorderStrong`): Table outer and header borders
        TableBorderStrong,
        /// Dear ImGui (`ImGuiCol_TableBorderLight`): Table inner borders
        TableBorderLight,
        /// Dear ImGui (`ImGuiCol_TableRowBg`): Table row background (even rows)
        TableRowBg,
        /// Dear ImGui (`ImGuiCol_TableRowBgAlt`): Table row background (odd rows)
        TableRowBgAlt,
        /// Dear ImGui (`ImGuiCol_TextLink`): Hyperlink color
        TextLink,
        /// Dear ImGui (`ImGuiCol_TextSelectedBg`): Selected text inside an InputText
        TextSelectedBg,
        /// Dear ImGui (`ImGuiCol_TreeLines`): Tree node hierarchy outlines
        TreeLines,
        /// Dear ImGui (`ImGuiCol_DragDropTarget`): Rectangle border highlighting a drop target
        DragDropTarget,
        /// Dear ImGui (`ImGuiCol_DragDropTargetBg`): Rectangle background highlighting a drop target
        DragDropTargetBg,
        /// Dear ImGui (`ImGuiCol_UnsavedMarker`): Unsaved marker color
        UnsavedMarker,
        /// Dear ImGui (`ImGuiCol_NavCursor`): Navigation cursor
        NavCursor,
        /// Dear ImGui (`ImGuiCol_NavWindowingHighlight`): Navigation windowing highlight
        NavWindowingHighlight,
        /// Dear ImGui (`ImGuiCol_NavWindowingDimBg`): Navigation windowing dim background
        NavWindowingDimBg,
        /// Dear ImGui (`ImGuiCol_ModalWindowDimBg`): Modal window dim background
        ModalWindowDimBg,
    }
}

imgui_enum! {
    /// Dear ImGui (`ImGuiStyleVar`): Variable identifier for styling
    pub StyleVar: ImGuiStyleVar_ {
        /// Dear ImGui (`ImGuiStyleVar_Alpha`): Global alpha
        Alpha,
        /// Dear ImGui (`ImGuiStyleVar_DisabledAlpha`): Disabled alpha
        DisabledAlpha,
        /// Dear ImGui (`ImGuiStyleVar_WindowPadding`): Window padding
        WindowPadding,
        /// Dear ImGui (`ImGuiStyleVar_WindowRounding`): Window rounding
        WindowRounding,
        /// Dear ImGui (`ImGuiStyleVar_WindowBorderSize`): Window border size
        WindowBorderSize,
        /// Dear ImGui (`ImGuiStyleVar_WindowMinSize`): Window min size
        WindowMinSize,
        /// Dear ImGui (`ImGuiStyleVar_WindowTitleAlign`): Window title align
        WindowTitleAlign,
        /// Dear ImGui (`ImGuiStyleVar_ChildRounding`): Child rounding
        ChildRounding,
        /// Dear ImGui (`ImGuiStyleVar_ChildBorderSize`): Child border size
        ChildBorderSize,
        /// Dear ImGui (`ImGuiStyleVar_PopupRounding`): Popup rounding
        PopupRounding,
        /// Dear ImGui (`ImGuiStyleVar_PopupBorderSize`): Popup border size
        PopupBorderSize,
        /// Dear ImGui (`ImGuiStyleVar_FramePadding`): Frame padding
        FramePadding,
        /// Dear ImGui (`ImGuiStyleVar_FrameRounding`): Frame rounding
        FrameRounding,
        /// Dear ImGui (`ImGuiStyleVar_FrameBorderSize`): Frame border size
        FrameBorderSize,
        /// Dear ImGui (`ImGuiStyleVar_ItemSpacing`): Item spacing
        ItemSpacing,
        /// Dear ImGui (`ImGuiStyleVar_ItemInnerSpacing`): Item inner spacing
        ItemInnerSpacing,
        /// Dear ImGui (`ImGuiStyleVar_IndentSpacing`): Indent spacing
        IndentSpacing,
        /// Dear ImGui (`ImGuiStyleVar_CellPadding`): Cell padding
        CellPadding,
        /// Dear ImGui (`ImGuiStyleVar_ScrollbarSize`): Scrollbar size
        ScrollbarSize,
        /// Dear ImGui (`ImGuiStyleVar_ScrollbarRounding`): Scrollbar rounding
        ScrollbarRounding,
        /// Dear ImGui (`ImGuiStyleVar_ScrollbarPadding`): Scrollbar padding
        ScrollbarPadding,
        /// Dear ImGui (`ImGuiStyleVar_GrabMinSize`): Grab min size
        GrabMinSize,
        /// Dear ImGui (`ImGuiStyleVar_GrabRounding`): Grab rounding
        GrabRounding,
        /// Dear ImGui (`ImGuiStyleVar_ImageRounding`): Image rounding
        ImageRounding,
        /// Dear ImGui (`ImGuiStyleVar_ImageBorderSize`): Image border size
        ImageBorderSize,
        /// Dear ImGui (`ImGuiStyleVar_TabRounding`): Tab rounding
        TabRounding,
        /// Dear ImGui (`ImGuiStyleVar_TabBorderSize`): Tab border size
        TabBorderSize,
        /// Dear ImGui (`ImGuiStyleVar_TabMinWidthBase`): Tab min width base
        TabMinWidthBase,
        /// Dear ImGui (`ImGuiStyleVar_TabMinWidthShrink`): Tab min width shrink
        TabMinWidthShrink,
        /// Dear ImGui (`ImGuiStyleVar_TabBarBorderSize`): Tab bar border size
        TabBarBorderSize,
        /// Dear ImGui (`ImGuiStyleVar_TabBarOverlineSize`): Tab bar overline size
        TabBarOverlineSize,
        /// Dear ImGui (`ImGuiStyleVar_TableAngledHeadersAngle`): Table angled headers angle
        TableAngledHeadersAngle,
        /// Dear ImGui (`ImGuiStyleVar_TableAngledHeadersTextAlign`): Table angled headers text align
        TableAngledHeadersTextAlign,
        /// Dear ImGui (`ImGuiStyleVar_TreeLinesSize`): Tree lines size
        TreeLinesSize,
        /// Dear ImGui (`ImGuiStyleVar_TreeLinesRounding`): Tree lines rounding
        TreeLinesRounding,
        /// Dear ImGui (`ImGuiStyleVar_DragDropTargetRounding`): Drag drop target rounding
        DragDropTargetRounding,
        /// Dear ImGui (`ImGuiStyleVar_ButtonTextAlign`): Button text align
        ButtonTextAlign,
        /// Dear ImGui (`ImGuiStyleVar_SelectableTextAlign`): Selectable text align
        SelectableTextAlign,
        /// Dear ImGui (`ImGuiStyleVar_SeparatorSize`): Separator size
        SeparatorSize,
        /// Dear ImGui (`ImGuiStyleVar_SeparatorTextBorderSize`): Separator text border size
        SeparatorTextBorderSize,
        /// Dear ImGui (`ImGuiStyleVar_SeparatorTextAlign`): Separator text align
        SeparatorTextAlign,
        /// Dear ImGui (`ImGuiStyleVar_SeparatorTextPadding`): Separator text padding
        SeparatorTextPadding,
        /// Dear ImGui (`ImGuiStyleVar_DockingSeparatorSize`): Docking separator size
        DockingSeparatorSize,
        /// Dear ImGui (`ImGuiStyleVar_MenuItemRounding`): Radius of MenuItem, BeginMenu rounding
        MenuItemRounding,
        /// Dear ImGui (`ImGuiStyleVar_SelectableRounding`): Radius of Selectable rounding
        SelectableRounding,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiWindowFlags`): Flags for `Begin()` and `BeginChild()`
    pub WindowFlags: ImGuiWindowFlags_ {
        /// Dear ImGui (`ImGuiWindowFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiWindowFlags_NoTitleBar`): Disable title-bar
        NoTitleBar,
        /// Dear ImGui (`ImGuiWindowFlags_NoResize`): Disable user resizing with the lower-right grip
        NoResize,
        /// Dear ImGui (`ImGuiWindowFlags_NoMove`): Disable user moving the window
        NoMove,
        /// Dear ImGui (`ImGuiWindowFlags_NoScrollbar`): Disable scrollbars (window can still scroll with mouse or programmatically)
        NoScrollbar,
        /// Dear ImGui (`ImGuiWindowFlags_NoScrollWithMouse`): Disable user vertically scrolling with mouse wheel. On child window, mouse wheel will be forwarded to the parent unless NoScrollbar is also set.
        NoScrollWithMouse,
        /// Dear ImGui (`ImGuiWindowFlags_NoCollapse`): Disable user collapsing window by double-clicking on it. Also referred to as Window Menu Button (e.g. within a docking node).
        NoCollapse,
        /// Dear ImGui (`ImGuiWindowFlags_AlwaysAutoResize`): Resize every window to its content every frame
        AlwaysAutoResize,
        /// Dear ImGui (`ImGuiWindowFlags_NoBackground`): Disable drawing background color (WindowBg, etc.) and outside border. Similar as using SetNextWindowBgAlpha(0.0f).
        NoBackground,
        /// Dear ImGui (`ImGuiWindowFlags_NoSavedSettings`): Never load/save settings in .ini file
        NoSavedSettings,
        /// Dear ImGui (`ImGuiWindowFlags_NoMouseInputs`): Disable catching mouse, hovering test with pass through.
        NoMouseInputs,
        /// Dear ImGui (`ImGuiWindowFlags_MenuBar`): Has a menu-bar
        MenuBar,
        /// Dear ImGui (`ImGuiWindowFlags_HorizontalScrollbar`): Allow horizontal scrollbar to appear (off by default). You may use SetNextWindowContentSize(ImVec2(width,0.0f)); prior to calling Begin() to specify width.
        HorizontalScrollbar,
        /// Dear ImGui (`ImGuiWindowFlags_NoFocusOnAppearing`): Disable taking focus when transitioning from hidden to visible state
        NoFocusOnAppearing,
        /// Dear ImGui (`ImGuiWindowFlags_NoBringToFrontOnFocus`): Disable bringing window to front when taking focus (e.g. clicking on it or programmatically giving it focus)
        NoBringToFrontOnFocus,
        /// Dear ImGui (`ImGuiWindowFlags_AlwaysVerticalScrollbar`): Always show vertical scrollbar (even if ContentSize.y < Size.y)
        AlwaysVerticalScrollbar,
        /// Dear ImGui (`ImGuiWindowFlags_AlwaysHorizontalScrollbar`): Always show horizontal scrollbar (even if ContentSize.x < Size.x)
        AlwaysHorizontalScrollbar,
        /// Dear ImGui (`ImGuiWindowFlags_NoNavInputs`): No keyboard/gamepad navigation within the window
        NoNavInputs,
        /// Dear ImGui (`ImGuiWindowFlags_NoNavFocus`): No focusing toward this window with keyboard/gamepad navigation (e.g. skipped by Ctrl+Tab)
        NoNavFocus,
        /// Dear ImGui (`ImGuiWindowFlags_UnsavedDocument`): Display a dot next to the title. When used in a tab/docking context, tab is selected when clicking the X + closure is not assumed (will wait for user to stop submitting the tab).
        UnsavedDocument,
        /// Dear ImGui (`ImGuiWindowFlags_NoDocking`): Disable docking of this window
        NoDocking,
        /// Dear ImGui (`ImGuiWindowFlags_NoNav`): `ImGuiWindowFlags_NoNavInputs` | `ImGuiWindowFlags_NoNavFocus`
        NoNav,
        /// Dear ImGui (`ImGuiWindowFlags_NoDecoration`): `ImGuiWindowFlags_NoTitleBar` | `ImGuiWindowFlags_NoResize` | `ImGuiWindowFlags_NoScrollbar` | `ImGuiWindowFlags_NoCollapse`
        NoDecoration,
        /// Dear ImGui (`ImGuiWindowFlags_NoInputs`): `ImGuiWindowFlags_NoMouseInputs` | `ImGuiWindowFlags_NoNavInputs` | `ImGuiWindowFlags_NoNavFocus`
        NoInputs,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiChildFlags`): Flags for `BeginChild()`
    pub ChildFlags: ImGuiChildFlags_ {
        /// Dear ImGui (`ImGuiChildFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiChildFlags_Borders`): Show an outer border and enable WindowPadding. (IMPORTANT: this is always == 1 == true for legacy reason)
        Borders,
        /// Dear ImGui (`ImGuiChildFlags_AlwaysUseWindowPadding`): Pad with style.WindowPadding even if no border are drawn
        AlwaysUseWindowPadding,
        /// Dear ImGui (`ImGuiChildFlags_ResizeX`): Allow resize from right border (layout direction). Enable .ini saving (unless ImGuiWindowFlags_NoSavedSettings passed to window flags)
        ResizeX,
        /// Dear ImGui (`ImGuiChildFlags_ResizeY`): Allow resize from bottom border (layout direction).
        ResizeY,
        /// Dear ImGui (`ImGuiChildFlags_AutoResizeX`): Enable auto-resizing width. Read "IMPORTANT: Size measurement" details above.
        AutoResizeX,
        /// Dear ImGui (`ImGuiChildFlags_AutoResizeY`): Enable auto-resizing height. Read "IMPORTANT: Size measurement" details above.
        AutoResizeY,
        /// Dear ImGui (`ImGuiChildFlags_AlwaysAutoResize`): Combined with AutoResizeX/AutoResizeY. Always measure size even when child is hidden, always return true, always disable clipping optimization! NOT RECOMMENDED.
        AlwaysAutoResize,
        /// Dear ImGui (`ImGuiChildFlags_FrameStyle`): Style the child window like a framed item: use FrameBg, FrameRounding, FrameBorderSize, FramePadding instead of ChildBg, ChildRounding, ChildBorderSize, WindowPadding.
        FrameStyle,
        /// Dear ImGui (`ImGuiChildFlags_NavFlattened`): [BETA] Share focus scope, allow keyboard/gamepad navigation to cross over parent border to this child or between sibling child windows.
        NavFlattened,
    }
}
imgui_flags! {
    /// Dear ImGui (`ImGuiButtonFlags`): Flags for `InvisibleButton()`
    pub ButtonFlags: ImGuiButtonFlags_ {
        /// Dear ImGui (`ImGuiButtonFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiButtonFlags_MouseButtonLeft`): React on left mouse button (default)
        MouseButtonLeft,
        /// Dear ImGui (`ImGuiButtonFlags_MouseButtonRight`): React on right mouse button
        MouseButtonRight,
        /// Dear ImGui (`ImGuiButtonFlags_MouseButtonMiddle`): React on center mouse button
        MouseButtonMiddle,
        /// Dear ImGui (`ImGuiButtonFlags_EnableNav`): InvisibleButton(): do not disable navigation/tabbing. Otherwise disabled by default.
        EnableNav,
        /// Dear ImGui (`ImGuiButtonFlags_AllowOverlap`): Hit testing will allow subsequent widgets to overlap this one. Require previous frame HoveredId to match before being usable. Shortcut to calling SetNextItemAllowOverlap().
        AllowOverlap,
    }
}

imgui_scoped_enum! {
    pub Dir: ImGuiDir {
        /// Dear ImGui (`ImGuiDir_Left`): Left
        Left,
        /// Dear ImGui (`ImGuiDir_Right`): Right
        Right,
        /// Dear ImGui (`ImGuiDir_Up`): Up
        Up,
        /// Dear ImGui (`ImGuiDir_Down`): Down
        Down,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiComboFlags`): Flags for `BeginCombo()`
    pub ComboFlags: ImGuiComboFlags_ {
        /// Dear ImGui (`ImGuiComboFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiComboFlags_PopupAlignLeft`): Align popup left
        PopupAlignLeft,
        /// Dear ImGui (`ImGuiComboFlags_HeightSmall`): Height small
        HeightSmall,
        /// Dear ImGui (`ImGuiComboFlags_HeightRegular`): Height regular
        HeightRegular,
        /// Dear ImGui (`ImGuiComboFlags_HeightLarge`): Height large
        HeightLarge,
        /// Dear ImGui (`ImGuiComboFlags_HeightLargest`): Height largest
        HeightLargest,
        /// Dear ImGui (`ImGuiComboFlags_NoArrowButton`): No arrow button
        NoArrowButton,
        /// Dear ImGui (`ImGuiComboFlags_NoPreview`): No preview
        NoPreview,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiSelectableFlags`): Flags for `Selectable()`
    pub SelectableFlags: ImGuiSelectableFlags_ {
        /// Dear ImGui (`ImGuiSelectableFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiSelectableFlags_NoAutoClosePopups`): Do not close popup when clicked
        NoAutoClosePopups,
        /// Dear ImGui (`ImGuiSelectableFlags_SpanAllColumns`): Span all columns
        SpanAllColumns,
        /// Dear ImGui (`ImGuiSelectableFlags_AllowDoubleClick`): Allow double click
        AllowDoubleClick,
        /// Dear ImGui (`ImGuiSelectableFlags_Disabled`): Disabled
        Disabled,
        /// Dear ImGui (`ImGuiSelectableFlags_AllowOverlap`): Allow overlap
        AllowOverlap,
        /// Dear ImGui (`ImGuiSelectableFlags_Highlight`): Highlight
        Highlight,
        /// Dear ImGui (`ImGuiSelectableFlags_SelectOnNav`): Select on nav
        SelectOnNav,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiSliderFlags`): Flags for `DragFloat()`, `DragInt()`, `SliderFloat()`, `SliderInt()`, etc.
    pub SliderFlags: ImGuiSliderFlags_ {
        /// Dear ImGui (`ImGuiSliderFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiSliderFlags_Logarithmic`): Logarithmic
        Logarithmic,
        /// Dear ImGui (`ImGuiSliderFlags_NoRoundToFormat`): No round to format
        NoRoundToFormat,
        /// Dear ImGui (`ImGuiSliderFlags_NoInput`): No input
        NoInput,
        /// Dear ImGui (`ImGuiSliderFlags_WrapAround`): Wrap around
        WrapAround,
        /// Dear ImGui (`ImGuiSliderFlags_ClampOnInput`): Clamp on input
        ClampOnInput,
        /// Dear ImGui (`ImGuiSliderFlags_ClampZeroRange`): Clamp zero range
        ClampZeroRange,
        /// Dear ImGui (`ImGuiSliderFlags_NoSpeedTweaks`): No speed tweaks
        NoSpeedTweaks,
        /// Dear ImGui (`ImGuiSliderFlags_ColorMarkers`): Color markers
        ColorMarkers,
        /// Dear ImGui (`ImGuiSliderFlags_AlwaysClamp`): Always clamp
        AlwaysClamp,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiInputTextFlags`): Flags for `InputText()`, `InputTextMultiline()`
    pub InputTextFlags: ImGuiInputTextFlags_ {
        // Basic filters
        /// Dear ImGui (`ImGuiInputTextFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiInputTextFlags_CharsDecimal`): Allow 0123456789.
        CharsDecimal,
        /// Dear ImGui (`ImGuiInputTextFlags_CharsHexadecimal`): Allow 0123456789ABCDEFabcdef
        CharsHexadecimal,
        /// Dear ImGui (`ImGuiInputTextFlags_CharsScientific`): Allow 0123456789.eE+-
        CharsScientific,
        /// Dear ImGui (`ImGuiInputTextFlags_CharsUppercase`): Turn character into upper case
        CharsUppercase,
        /// Dear ImGui (`ImGuiInputTextFlags_CharsNoBlank`): Filter out spaces
        CharsNoBlank,

        // Inputs
        /// Dear ImGui (`ImGuiInputTextFlags_AllowTabInput`): Tab key enters a tab character
        AllowTabInput,
        /// Dear ImGui (`ImGuiInputTextFlags_EnterReturnsTrue`): Return 'true' when Enter is pressed
        EnterReturnsTrue,
        /// Dear ImGui (`ImGuiInputTextFlags_EscapeClearsAll`): Escape clears input
        EscapeClearsAll,
        /// Dear ImGui (`ImGuiInputTextFlags_CtrlEnterForNewLine`): Ctrl+Enter adds a new line
        CtrlEnterForNewLine,

        // Other options
        /// Dear ImGui (`ImGuiInputTextFlags_ReadOnly`): Read-only mode
        ReadOnly,
        /// Dear ImGui (`ImGuiInputTextFlags_Password`): Password mode (mask characters)
        Password,
        /// Dear ImGui (`ImGuiInputTextFlags_AlwaysOverwrite`): Always overwrite mode
        AlwaysOverwrite,
        /// Dear ImGui (`ImGuiInputTextFlags_AutoSelectAll`): Auto-select all on focus
        AutoSelectAll,
        /// Dear ImGui (`ImGuiInputTextFlags_ParseEmptyRefVal`): Parse empty reference value
        ParseEmptyRefVal,
        /// Dear ImGui (`ImGuiInputTextFlags_DisplayEmptyRefVal`): Display empty reference value
        DisplayEmptyRefVal,
        /// Dear ImGui (`ImGuiInputTextFlags_NoHorizontalScroll`): No horizontal scroll
        NoHorizontalScroll,
        /// Dear ImGui (`ImGuiInputTextFlags_NoUndoRedo`): No undo/redo
        NoUndoRedo,

        // Elide display / Alignment
        /// Dear ImGui (`ImGuiInputTextFlags_ElideLeft`): Elide left
        ElideLeft,

        // Callback features
        /// Dear ImGui (`ImGuiInputTextFlags_CallbackCompletion`): Callback on completion
        CallbackCompletion,
        /// Dear ImGui (`ImGuiInputTextFlags_CallbackHistory`): Callback on history
        CallbackHistory,
        /// Dear ImGui (`ImGuiInputTextFlags_CallbackAlways`): Callback always
        CallbackAlways,
        /// Dear ImGui (`ImGuiInputTextFlags_CallbackCharFilter`): Callback on character filter
        CallbackCharFilter,
        /// Dear ImGui (`ImGuiInputTextFlags_CallbackResize`): Callback on resize
        CallbackResize,
        /// Dear ImGui (`ImGuiInputTextFlags_CallbackEdit`): Callback on edit
        CallbackEdit,

        // Multi-line Word-Wrapping [BETA]
        /// Dear ImGui (`ImGuiInputTextFlags_WordWrap`): Word-wrap
        WordWrap,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiHoveredFlags`): Flags for `IsItemHovered()`, `IsWindowHovered()` etc.
    pub HoveredFlags: ImGuiHoveredFlags_ {
        /// Dear ImGui (`ImGuiHoveredFlags_None`): Return true if directly over the item/window, not obstructed by another window, not obstructed by an active popup or modal blocking inputs under them.
        None,
        /// Dear ImGui (`ImGuiHoveredFlags_ChildWindows`): IsWindowHovered() only: Return true if any children of the window is hovered
        ChildWindows,
        /// Dear ImGui (`ImGuiHoveredFlags_RootWindow`): IsWindowHovered() only: Test from root window (top most parent of the current hierarchy)
        RootWindow,
        /// Dear ImGui (`ImGuiHoveredFlags_AnyWindow`): IsWindowHovered() only: Return true if any window is hovered
        AnyWindow,
        /// Dear ImGui (`ImGuiHoveredFlags_NoPopupHierarchy`): IsWindowHovered() only: Do not consider popup hierarchy (do not treat popup emitter as parent of popup) (when used with _ChildWindows or _RootWindow)
        NoPopupHierarchy,
        /// Dear ImGui (`ImGuiHoveredFlags_DockHierarchy`): IsWindowHovered() only: Consider docking hierarchy (treat dockspace host as parent of docked window) (when used with _ChildWindows or _RootWindow)
        DockHierarchy,
        /// Dear ImGui (`ImGuiHoveredFlags_AllowWhenBlockedByPopup`): Return true even if a popup window is normally blocking access to this item/window
        AllowWhenBlockedByPopup,
        /// Dear ImGui (`ImGuiHoveredFlags_AllowWhenBlockedByActiveItem`): Return true even if an active item is blocking access to this item/window. Useful for Drag and Drop patterns.
        AllowWhenBlockedByActiveItem,
        /// Dear ImGui (`ImGuiHoveredFlags_AllowWhenOverlappedByItem`): IsItemHovered() only: Return true even if the item uses AllowOverlap mode and is overlapped by another hoverable item.
        AllowWhenOverlappedByItem,
        /// Dear ImGui (`ImGuiHoveredFlags_AllowWhenOverlappedByWindow`): IsItemHovered() only: Return true even if the position is obstructed or overlapped by another window.
        AllowWhenOverlappedByWindow,
        /// Dear ImGui (`ImGuiHoveredFlags_AllowWhenDisabled`): IsItemHovered() only: Return true even if the item is disabled
        AllowWhenDisabled,
        /// Dear ImGui (`ImGuiHoveredFlags_NoNavOverride`): IsItemHovered() only: Disable using keyboard/gamepad navigation state when active, always query mouse
        NoNavOverride,
        /// Dear ImGui (`ImGuiHoveredFlags_AllowWhenOverlapped`): Allow when overlapped
        AllowWhenOverlapped,
        /// Dear ImGui (`ImGuiHoveredFlags_RectOnly`): Rect only
        RectOnly,
        /// Dear ImGui (`ImGuiHoveredFlags_RootAndChildWindows`): Root and child windows
        RootAndChildWindows,
        /// Dear ImGui (`ImGuiHoveredFlags_ForTooltip`): For tooltip
        ForTooltip,
        /// Dear ImGui (`ImGuiHoveredFlags_Stationary`): Stationary
        Stationary,
        /// Dear ImGui (`ImGuiHoveredFlags_DelayNone`): Delay none
        DelayNone,
        /// Dear ImGui (`ImGuiHoveredFlags_DelayShort`): Delay short
        DelayShort,
        /// Dear ImGui (`ImGuiHoveredFlags_DelayNormal`): Delay normal
        DelayNormal,
        /// Dear ImGui (`ImGuiHoveredFlags_NoSharedDelay`): No shared delay
        NoSharedDelay,
    }
}

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u16),
}

impl MouseButton {
    pub fn bits(self) -> i32 {
        match self {
            MouseButton::Left => ImGuiMouseButton_::ImGuiMouseButton_Left.0 as i32,
            MouseButton::Right => ImGuiMouseButton_::ImGuiMouseButton_Right.0 as i32,
            MouseButton::Middle => ImGuiMouseButton_::ImGuiMouseButton_Middle.0 as i32,
            MouseButton::Other(x) => x as i32,
        }
    }
}

imgui_enum! {
    /// Dear ImGui (`ImGuiMouseCursor`): Mouse cursor shape
    pub MouseCursor : ImGuiMouseCursor_ {
        /// Dear ImGui (`ImGuiMouseCursor_None`): No cursor
        None,
        /// Dear ImGui (`ImGuiMouseCursor_Arrow`): Arrow
        Arrow,
        /// Dear ImGui (`ImGuiMouseCursor_TextInput`): When hovering over InputText, etc.
        TextInput,
        /// Dear ImGui (`ImGuiMouseCursor_ResizeAll`): (Unused by Dear ImGui functions)
        ResizeAll,
        /// Dear ImGui (`ImGuiMouseCursor_ResizeNS`): When hovering over a horizontal border
        ResizeNS,
        /// Dear ImGui (`ImGuiMouseCursor_ResizeEW`): When hovering over a vertical border or a column
        ResizeEW,
        /// Dear ImGui (`ImGuiMouseCursor_ResizeNESW`): When hovering over the bottom-left corner of a window
        ResizeNESW,
        /// Dear ImGui (`ImGuiMouseCursor_ResizeNWSE`): When hovering over the bottom-right corner of a window
        ResizeNWSE,
        /// Dear ImGui (`ImGuiMouseCursor_Hand`): (Unused by Dear ImGui functions. Use for e.g. hyperlinks)
        Hand,
        /// Dear ImGui (`ImGuiMouseCursor_NotAllowed`): When hovering something with disallowed interaction
        NotAllowed,
    }
}

// ImGuiKey is named weirdly
impl_bit_enum_helper! {ImGuiKey}

imgui_enum_ex! {
    pub Key: ImGuiKey: ImGuiKey {
        None = ImGuiKey_None,
        Tab = ImGuiKey_Tab,
        LeftArrow = ImGuiKey_LeftArrow,
        RightArrow = ImGuiKey_RightArrow,
        UpArrow = ImGuiKey_UpArrow,
        DownArrow = ImGuiKey_DownArrow,
        PageUp = ImGuiKey_PageUp,
        PageDown = ImGuiKey_PageDown,
        Home = ImGuiKey_Home,
        End = ImGuiKey_End,
        Insert = ImGuiKey_Insert,
        Delete = ImGuiKey_Delete,
        Backspace = ImGuiKey_Backspace,
        Space = ImGuiKey_Space,
        Enter = ImGuiKey_Enter,
        Escape = ImGuiKey_Escape,
        LeftCtrl = ImGuiKey_LeftCtrl,
        LeftShift = ImGuiKey_LeftShift,
        LeftAlt = ImGuiKey_LeftAlt,
        LeftSuper = ImGuiKey_LeftSuper,
        RightCtrl = ImGuiKey_RightCtrl,
        RightShift = ImGuiKey_RightShift,
        RightAlt = ImGuiKey_RightAlt,
        RightSuper = ImGuiKey_RightSuper,
        Menu = ImGuiKey_Menu,
        Num0 = ImGuiKey_0,
        Num1 = ImGuiKey_1,
        Num2 = ImGuiKey_2,
        Num3 = ImGuiKey_3,
        Num4 = ImGuiKey_4,
        Num5 = ImGuiKey_5,
        Num6 = ImGuiKey_6,
        Num7 = ImGuiKey_7,
        Num8 = ImGuiKey_8,
        Num9 = ImGuiKey_9,
        A = ImGuiKey_A,
        B = ImGuiKey_B,
        C = ImGuiKey_C,
        D = ImGuiKey_D,
        E = ImGuiKey_E,
        F = ImGuiKey_F,
        G = ImGuiKey_G,
        H = ImGuiKey_H,
        I = ImGuiKey_I,
        J = ImGuiKey_J,
        K = ImGuiKey_K,
        L = ImGuiKey_L,
        M = ImGuiKey_M,
        N = ImGuiKey_N,
        O = ImGuiKey_O,
        P = ImGuiKey_P,
        Q = ImGuiKey_Q,
        R = ImGuiKey_R,
        S = ImGuiKey_S,
        T = ImGuiKey_T,
        U = ImGuiKey_U,
        V = ImGuiKey_V,
        W = ImGuiKey_W,
        X = ImGuiKey_X,
        Y = ImGuiKey_Y,
        Z = ImGuiKey_Z,
        F1 = ImGuiKey_F1,
        F2 = ImGuiKey_F2,
        F3 = ImGuiKey_F3,
        F4 = ImGuiKey_F4,
        F5 = ImGuiKey_F5,
        F6 = ImGuiKey_F6,
        F7 = ImGuiKey_F7,
        F8 = ImGuiKey_F8,
        F9 = ImGuiKey_F9,
        F10 = ImGuiKey_F10,
        F11 = ImGuiKey_F11,
        F12 = ImGuiKey_F12,
        Apostrophe = ImGuiKey_Apostrophe,
        Comma = ImGuiKey_Comma,
        Minus = ImGuiKey_Minus,
        Period = ImGuiKey_Period,
        Slash = ImGuiKey_Slash,
        Semicolon = ImGuiKey_Semicolon,
        Equal = ImGuiKey_Equal,
        LeftBracket = ImGuiKey_LeftBracket,
        Backslash = ImGuiKey_Backslash,
        RightBracket = ImGuiKey_RightBracket,
        GraveAccent = ImGuiKey_GraveAccent,
        CapsLock = ImGuiKey_CapsLock,
        ScrollLock = ImGuiKey_ScrollLock,
        NumLock = ImGuiKey_NumLock,
        PrintScreen = ImGuiKey_PrintScreen,
        Pause = ImGuiKey_Pause,
        Keypad0 = ImGuiKey_Keypad0,
        Keypad1 = ImGuiKey_Keypad1,
        Keypad2 = ImGuiKey_Keypad2,
        Keypad3 = ImGuiKey_Keypad3,
        Keypad4 = ImGuiKey_Keypad4,
        Keypad5 = ImGuiKey_Keypad5,
        Keypad6 = ImGuiKey_Keypad6,
        Keypad7 = ImGuiKey_Keypad7,
        Keypad8 = ImGuiKey_Keypad8,
        Keypad9 = ImGuiKey_Keypad9,
        KeypadDecimal = ImGuiKey_KeypadDecimal,
        KeypadDivide = ImGuiKey_KeypadDivide,
        KeypadMultiply = ImGuiKey_KeypadMultiply,
        KeypadSubtract = ImGuiKey_KeypadSubtract,
        KeypadAdd = ImGuiKey_KeypadAdd,
        KeypadEnter = ImGuiKey_KeypadEnter,
        KeypadEqual = ImGuiKey_KeypadEqual,
        AppBack = ImGuiKey_AppBack,
        AppForward = ImGuiKey_AppForward,
        Oem102 = ImGuiKey_Oem102,

        GamepadStart = ImGuiKey_GamepadStart,
        GamepadBack = ImGuiKey_GamepadBack,
        GamepadFaceLeft = ImGuiKey_GamepadFaceLeft,
        GamepadFaceRight = ImGuiKey_GamepadFaceRight,
        GamepadFaceUp = ImGuiKey_GamepadFaceUp,
        GamepadFaceDown = ImGuiKey_GamepadFaceDown,
        GamepadDpadLeft = ImGuiKey_GamepadDpadLeft,
        GamepadDpadRight = ImGuiKey_GamepadDpadRight,
        GamepadDpadUp = ImGuiKey_GamepadDpadUp,
        GamepadDpadDown = ImGuiKey_GamepadDpadDown,
        GamepadL1 = ImGuiKey_GamepadL1,
        GamepadR1 = ImGuiKey_GamepadR1,
        GamepadL2 = ImGuiKey_GamepadL2,
        GamepadR2 = ImGuiKey_GamepadR2,
        GamepadL3 = ImGuiKey_GamepadL3,
        GamepadR3 = ImGuiKey_GamepadR3,
        GamepadLStickLeft = ImGuiKey_GamepadLStickLeft,
        GamepadLStickRight = ImGuiKey_GamepadLStickRight,
        GamepadLStickUp = ImGuiKey_GamepadLStickUp,
        GamepadLStickDown = ImGuiKey_GamepadLStickDown,
        GamepadRStickLeft = ImGuiKey_GamepadRStickLeft,
        GamepadRStickRight = ImGuiKey_GamepadRStickRight,
        GamepadRStickUp = ImGuiKey_GamepadRStickUp,
        GamepadRStickDown = ImGuiKey_GamepadRStickDown,

        MouseLeft = ImGuiKey_MouseLeft,
        MouseRight = ImGuiKey_MouseRight,
        MouseMiddle = ImGuiKey_MouseMiddle,
        MouseX1 = ImGuiKey_MouseX1,
        MouseX2 = ImGuiKey_MouseX2,
        MouseWheelX = ImGuiKey_MouseWheelX,
        MouseWheelY = ImGuiKey_MouseWheelY,

        // These are better handled as KeyMod, but sometimes can be seen as regular keys.
        ModCtrl = ImGuiMod_Ctrl,
        ModShift = ImGuiMod_Shift,
        ModAlt = ImGuiMod_Alt,
        ModSuper = ImGuiMod_Super,
    }
}

// ImGuiMod is not a real enum in the .h but are part of ImGuiKey.
// We week them separated because they can be OR-combined with keys and between them.
imgui_flags_ex! {
    pub KeyMod: ImGuiKey {
        None = ImGuiMod_None,
        Ctrl = ImGuiMod_Ctrl,
        Shift = ImGuiMod_Shift,
        Alt = ImGuiMod_Alt,
        Super = ImGuiMod_Super,
    }
}

impl TryFrom<Key> for KeyMod {
    type Error = ();
    fn try_from(key: Key) -> Result<KeyMod, Self::Error> {
        match key {
            Key::ModCtrl => Ok(KeyMod::Ctrl),
            Key::ModShift => Ok(KeyMod::Shift),
            Key::ModAlt => Ok(KeyMod::Alt),
            Key::ModSuper => Ok(KeyMod::Super),
            // KeyMod is a bitflags, but only one can be converted to Key
            _ => Err(()),
        }
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiViewportFlags`): Flags for `ImGuiViewport`
    pub ViewportFlags: ImGuiViewportFlags_ {
        /// Dear ImGui (`ImGuiViewportFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiViewportFlags_IsPlatformWindow`): Is platform window
        IsPlatformWindow,
        /// Dear ImGui (`ImGuiViewportFlags_IsPlatformMonitor`): Is platform monitor
        IsPlatformMonitor,
        /// Dear ImGui (`ImGuiViewportFlags_OwnedByApp`): Owned by application
        OwnedByApp,
        /// Dear ImGui (`ImGuiViewportFlags_NoDecoration`): No decoration
        NoDecoration,
        /// Dear ImGui (`ImGuiViewportFlags_NoTaskBarIcon`): No task bar icon
        NoTaskBarIcon,
        /// Dear ImGui (`ImGuiViewportFlags_NoFocusOnAppearing`): No focus on appearing
        NoFocusOnAppearing,
        /// Dear ImGui (`ImGuiViewportFlags_NoFocusOnClick`): No focus on click
        NoFocusOnClick,
        /// Dear ImGui (`ImGuiViewportFlags_NoInputs`): No inputs
        NoInputs,
        /// Dear ImGui (`ImGuiViewportFlags_NoRendererClear`): No renderer clear
        NoRendererClear,
        /// Dear ImGui (`ImGuiViewportFlags_NoAutoMerge`): No auto merge
        NoAutoMerge,
        /// Dear ImGui (`ImGuiViewportFlags_TopMost`): Top most
        TopMost,
        /// Dear ImGui (`ImGuiViewportFlags_CanHostOtherWindows`): Can host other windows
        CanHostOtherWindows,
        /// Dear ImGui (`ImGuiViewportFlags_IsMinimized`): Is minimized
        IsMinimized,
        /// Dear ImGui (`ImGuiViewportFlags_IsFocused`): Is focused
        IsFocused,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiPopupFlags`): Flags for `OpenPopup*()`, `BeginPopupContext*()`, `IsPopupOpen()`
    pub PopupFlags: ImGuiPopupFlags_ {
        /// Dear ImGui (`ImGuiPopupFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiPopupFlags_MouseButtonLeft`): Left mouse button
        MouseButtonLeft,
        /// Dear ImGui (`ImGuiPopupFlags_MouseButtonRight`): Right mouse button
        MouseButtonRight,
        /// Dear ImGui (`ImGuiPopupFlags_MouseButtonMiddle`): Middle mouse button
        MouseButtonMiddle,
        /// Dear ImGui (`ImGuiPopupFlags_NoReopen`): No reopen
        NoReopen,
        /// Dear ImGui (`ImGuiPopupFlags_NoOpenOverExistingPopup`): No open over existing popup
        NoOpenOverExistingPopup,
        /// Dear ImGui (`ImGuiPopupFlags_NoOpenOverItems`): No open over items
        NoOpenOverItems,
        /// Dear ImGui (`ImGuiPopupFlags_AnyPopupId`): Any popup ID
        AnyPopupId,
        /// Dear ImGui (`ImGuiPopupFlags_AnyPopupLevel`): Any popup level
        AnyPopupLevel,
        /// Dear ImGui (`ImGuiPopupFlags_AnyPopup`): Any popup
        AnyPopup,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiConfigFlags`): Flags for `io.ConfigFlags`
    pub ConfigFlags: ImGuiConfigFlags_ {
        /// Dear ImGui (`ImGuiConfigFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiConfigFlags_NavEnableKeyboard`): Master keyboard navigation enable
        NavEnableKeyboard,
        /// Dear ImGui (`ImGuiConfigFlags_NavEnableGamepad`): Master gamepad navigation enable
        NavEnableGamepad,
        /// Dear ImGui (`ImGuiConfigFlags_NoMouse`): Instruct backends to not pass mouse events
        NoMouse,
        /// Dear ImGui (`ImGuiConfigFlags_NoMouseCursorChange`): Instruct backends to not change mouse cursor shape
        NoMouseCursorChange,
        /// Dear ImGui (`ImGuiConfigFlags_NoKeyboard`): Instruct backends to not pass keyboard events
        NoKeyboard,
        /// Dear ImGui (`ImGuiConfigFlags_DockingEnable`): Docking enable
        DockingEnable,
        /// Dear ImGui (`ImGuiConfigFlags_ViewportsEnable`): Viewports enable
        ViewportsEnable,
        /// Dear ImGui (`ImGuiConfigFlags_IsSRGB`): Renderer is using sRGB
        IsSRGB,
        /// Dear ImGui (`ImGuiConfigFlags_IsTouchScreen`): Is touch screen
        IsTouchScreen,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiTreeNodeFlags`): Flags for `TreeNode()`, `TreeNodeEx()`, `CollapsingHeader()`
    pub TreeNodeFlags: ImGuiTreeNodeFlags_ {
        /// Dear ImGui (`ImGuiTreeNodeFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiTreeNodeFlags_Selected`): Draw as selected
        Selected,
        /// Dear ImGui (`ImGuiTreeNodeFlags_Framed`): Draw frame with background (e.g. CollapsingHeader)
        Framed,
        /// Dear ImGui (`ImGuiTreeNodeFlags_AllowOverlap`): Hit testing to allow following items to be overlapped
        AllowOverlap,
        /// Dear ImGui (`ImGuiTreeNodeFlags_NoTreePushOnOpen`): Don't do a TreePush() when open (e.g. CollapsingHeader)
        NoTreePushOnOpen,
        /// Dear ImGui (`ImGuiTreeNodeFlags_NoAutoOpenOnLog`): Don't automatically and temporarily open node when logging
        NoAutoOpenOnLog,
        /// Dear ImGui (`ImGuiTreeNodeFlags_DefaultOpen`): Default to open
        DefaultOpen,
        /// Dear ImGui (`ImGuiTreeNodeFlags_OpenOnDoubleClick`): Need double-click to open node
        OpenOnDoubleClick,
        /// Dear ImGui (`ImGuiTreeNodeFlags_OpenOnArrow`): Only open when clicking on the arrow part
        OpenOnArrow,
        /// Dear ImGui (`ImGuiTreeNodeFlags_Leaf`): No collapsing, no arrow
        Leaf,
        /// Dear ImGui (`ImGuiTreeNodeFlags_Bullet`): Display a bullet instead of arrow
        Bullet,
        /// Dear ImGui (`ImGuiTreeNodeFlags_FramePadding`): Use `FramePadding` (even for TreeNodeEx)
        FramePadding,
        /// Dear ImGui (`ImGuiTreeNodeFlags_SpanAvailWidth`): Extend hit box to the right-most edge, even if not framed
        SpanAvailWidth,
        /// Dear ImGui (`ImGuiTreeNodeFlags_SpanFullWidth`): Extend hit box to the left-most and right-most edges
        SpanFullWidth,
        /// Dear ImGui (`ImGuiTreeNodeFlags_SpanLabelWidth`): Only hit test the label
        SpanLabelWidth,
        /// Dear ImGui (`ImGuiTreeNodeFlags_SpanAllColumns`): Span all columns
        SpanAllColumns,
        /// Dear ImGui (`ImGuiTreeNodeFlags_LabelSpanAllColumns`): Label span all columns
        LabelSpanAllColumns,
        /// Dear ImGui (`ImGuiTreeNodeFlags_NavLeftJumpsToParent`): Nav left jumps to parent
        NavLeftJumpsToParent,
        /// Dear ImGui (`ImGuiTreeNodeFlags_CollapsingHeader`): `ImGuiTreeNodeFlags_Framed` | `ImGuiTreeNodeFlags_NoTreePushOnOpen` | `ImGuiTreeNodeFlags_NoAutoOpenOnLog`
        CollapsingHeader,
        /// Dear ImGui (`ImGuiTreeNodeFlags_DrawLinesNone`): Draw lines none
        DrawLinesNone,
        /// Dear ImGui (`ImGuiTreeNodeFlags_DrawLinesFull`): Draw lines full
        DrawLinesFull,
        /// Dear ImGui (`ImGuiTreeNodeFlags_DrawLinesToNodes`): Draw lines to nodes
        DrawLinesToNodes,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiFocusedFlags`): Flags for `IsWindowFocused()`
    pub FocusedFlags: ImGuiFocusedFlags_ {
        /// Dear ImGui (`ImGuiFocusedFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiFocusedFlags_ChildWindows`): Return true if any child window is focused
        ChildWindows,
        /// Dear ImGui (`ImGuiFocusedFlags_RootWindow`): Test from root of the window hierarchy
        RootWindow,
        /// Dear ImGui (`ImGuiFocusedFlags_AnyWindow`): Return true if any window is focused
        AnyWindow,
        /// Dear ImGui (`ImGuiFocusedFlags_NoPopupHierarchy`): Do not test if popup hierarchy is focused
        NoPopupHierarchy,
        /// Dear ImGui (`ImGuiFocusedFlags_DockHierarchy`): Do not test if dock hierarchy is focused
        DockHierarchy,
        /// Dear ImGui (`ImGuiFocusedFlags_RootAndChildWindows`): Test from root and child windows
        RootAndChildWindows,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiColorEditFlags`): Flags for `ColorEdit4()`, `ColorPicker4()` etc.
    pub ColorEditFlags: ImGuiColorEditFlags_ {
        /// Dear ImGui (`ImGuiColorEditFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiColorEditFlags_NoAlpha`): ColorEdit, ColorPicker, ColorButton: ignore Alpha component (will only read 3 components from the input pointer).
        NoAlpha,
        /// Dear ImGui (`ImGuiColorEditFlags_NoPicker`): ColorEdit: disable picker when clicking on color square.
        NoPicker,
        /// Dear ImGui (`ImGuiColorEditFlags_NoOptions`): ColorEdit: disable toggling options menu when right-clicking on inputs/small preview.
        NoOptions,
        /// Dear ImGui (`ImGuiColorEditFlags_NoSmallPreview`): ColorEdit, ColorPicker: disable color square preview next to the inputs. (e.g. to show only the inputs)
        NoSmallPreview,
        /// Dear ImGui (`ImGuiColorEditFlags_NoInputs`): ColorEdit, ColorPicker: disable inputs sliders/text widgets (e.g. to show only the small preview color square).
        NoInputs,
        /// Dear ImGui (`ImGuiColorEditFlags_NoTooltip`): ColorEdit, ColorPicker, ColorButton: disable tooltip when hovering the preview.
        NoTooltip,
        /// Dear ImGui (`ImGuiColorEditFlags_NoLabel`): ColorEdit, ColorPicker: disable display of inline text label (the label is still forwarded to the tooltip and picker).
        NoLabel,
        /// Dear ImGui (`ImGuiColorEditFlags_NoSidePreview`): ColorPicker: disable bigger color preview on right side of the picker, use small color square preview instead.
        NoSidePreview,
        /// Dear ImGui (`ImGuiColorEditFlags_NoDragDrop`): ColorEdit: disable drag and drop target/source. ColorButton: disable drag and drop source.
        NoDragDrop,
        /// Dear ImGui (`ImGuiColorEditFlags_NoBorder`): ColorButton: disable border (which is enforced by default)
        NoBorder,
        /// Dear ImGui (`ImGuiColorEditFlags_NoColorMarkers`): ColorEdit: disable rendering R/G/B/A color marker.
        NoColorMarkers,
        /// Dear ImGui (`ImGuiColorEditFlags_AlphaOpaque`): ColorEdit, ColorPicker, ColorButton: disable alpha in the preview,. Contrary to _NoAlpha it may still be edited when calling ColorEdit4()/ColorPicker4().
        AlphaOpaque,
        /// Dear ImGui (`ImGuiColorEditFlags_AlphaNoBg`): ColorEdit, ColorPicker, ColorButton: disable rendering a checkerboard background behind transparent color.
        AlphaNoBg,
        /// Dear ImGui (`ImGuiColorEditFlags_AlphaPreviewHalf`): ColorEdit, ColorPicker, ColorButton: display half opaque / half transparent preview.
        AlphaPreviewHalf,
        /// Dear ImGui (`ImGuiColorEditFlags_AlphaBar`): ColorEdit, ColorPicker: show vertical alpha bar/gradient in picker.
        AlphaBar,
        /// Dear ImGui (`ImGuiColorEditFlags_HDR`): (WIP) ColorEdit: Currently only disable 0.0f..1.0f limits in RGBA edition (note: you probably want to use ImGuiColorEditFlags_Float flag as well).
        HDR,
        /// Dear ImGui (`ImGuiColorEditFlags_DisplayRGB`): ColorEdit: override _display_ type among RGB/HSV/Hex. ColorPicker: select any combination using one or more of RGB/HSV/Hex.
        DisplayRGB,
        /// Dear ImGui (`ImGuiColorEditFlags_DisplayHSV`): [Display] // "
        DisplayHSV,
        /// Dear ImGui (`ImGuiColorEditFlags_DisplayHex`): [Display] // "
        DisplayHex,
        /// Dear ImGui (`ImGuiColorEditFlags_Uint8`): ColorEdit, ColorPicker, ColorButton: _display_ values formatted as 0..255.
        Uint8,
        /// Dear ImGui (`ImGuiColorEditFlags_Float`): ColorEdit, ColorPicker, ColorButton: _display_ values formatted as 0.0f..1.0f floats instead of 0..255 integers.
        Float,
        /// Dear ImGui (`ImGuiColorEditFlags_PickerHueBar`): ColorPicker: bar for Hue, rectangle for Sat/Value.
        PickerHueBar,
        /// Dear ImGui (`ImGuiColorEditFlags_PickerHueWheel`): ColorPicker: wheel for Hue, triangle for Sat/Value.
        PickerHueWheel,
        /// Dear ImGui (`ImGuiColorEditFlags_PickerNoRotate`): ColorPicker: disable rotating Sat/Value triangle
        PickerNoRotate,
        /// Dear ImGui (`ImGuiColorEditFlags_InputRGB`): ColorEdit, ColorPicker: input and output data in RGB format.
        InputRGB,
        /// Dear ImGui (`ImGuiColorEditFlags_InputHSV`): ColorEdit, ColorPicker: input and output data in HSV format.
        InputHSV,
        /// Dear ImGui (`ImGuiColorEditFlags_DefaultOptions_`): Default options
        DefaultOptions_,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiTabBarFlags`): Flags for `BeginTabBar()`
    pub TabBarFlags: ImGuiTabBarFlags_ {
        /// Dear ImGui (`ImGuiTabBarFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiTabBarFlags_Reorderable`): Allow reordering of tabs
        Reorderable,
        /// Dear ImGui (`ImGuiTabBarFlags_AutoSelectNewTabs`): Auto-select new tabs
        AutoSelectNewTabs,
        /// Dear ImGui (`ImGuiTabBarFlags_TabListPopupButton`): Tab list popup button
        TabListPopupButton,
        /// Dear ImGui (`ImGuiTabBarFlags_NoCloseWithMiddleMouseButton`): No close with middle mouse button
        NoCloseWithMiddleMouseButton,
        /// Dear ImGui (`ImGuiTabBarFlags_NoTabListScrollingButtons`): Disable scrolling buttons (e.g. arrows)
        NoTabListScrollingButtons,
        /// Dear ImGui (`ImGuiTabBarFlags_NoTooltip`): Disable tooltips when hovering a tab
        NoTooltip,
        /// Dear ImGui (`ImGuiTabBarFlags_DrawSelectedOverline`): Draw a horizontal line under the selected tab.
        DrawSelectedOverline,
        /// Dear ImGui (`ImGuiTabBarFlags_FittingPolicyMixed`): Growing tabs: automatically resize tabs to fit in width
        FittingPolicyMixed,
        /// Dear ImGui (`ImGuiTabBarFlags_FittingPolicyShrink`): Shrink tabs to fit in width
        FittingPolicyShrink,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiTabItemFlags`): Flags for `BeginTabItem()`
    pub TabItemFlags: ImGuiTabItemFlags_ {
        /// Dear ImGui (`ImGuiTabItemFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiTabItemFlags_UnsavedDocument`): Append '*' to title
        UnsavedDocument,
        /// Dear ImGui (`ImGuiTabItemFlags_SetSelected`): Set selected
        SetSelected,
        /// Dear ImGui (`ImGuiTabItemFlags_NoCloseWithMiddleMouseButton`): No close with middle mouse button
        NoCloseWithMiddleMouseButton,
        /// Dear ImGui (`ImGuiTabItemFlags_NoPushId`): Don't call PushID(tab->ID)/PopID() on tab items
        NoPushId,
        /// Dear ImGui (`ImGuiTabItemFlags_NoTooltip`): Disable tooltip for the given tab
        NoTooltip,
        /// Dear ImGui (`ImGuiTabItemFlags_NoReorder`): Disable reordering this tab or specifying some variables
        NoReorder,
        /// Dear ImGui (`ImGuiTabItemFlags_Leading`): Enforce the tab position to the left of the tab bar (after the tab list popup button)
        Leading,
        /// Dear ImGui (`ImGuiTabItemFlags_Trailing`): Enforce the tab position to the right of the tab bar (before the scrolling buttons)
        Trailing,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiBackendFlags`): Flags for `io.BackendFlags`
    pub BackendFlags: ImGuiBackendFlags_ {
        /// Dear ImGui (`ImGuiBackendFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiBackendFlags_HasGamepad`): Backend has gamepad
        HasGamepad,
        /// Dear ImGui (`ImGuiBackendFlags_HasMouseCursors`): Backend has mouse cursors
        HasMouseCursors,
        /// Dear ImGui (`ImGuiBackendFlags_HasSetMousePos`): Backend can set mouse position
        HasSetMousePos,
        /// Dear ImGui (`ImGuiBackendFlags_RendererHasVtxOffset`): Backend renderer has vertex offset
        RendererHasVtxOffset,
        /// Dear ImGui (`ImGuiBackendFlags_RendererHasTextures`): Backend renderer has textures
        RendererHasTextures,
        /// Dear ImGui (`ImGuiBackendFlags_RendererHasViewports`): Backend renderer has viewports
        RendererHasViewports,
        /// Dear ImGui (`ImGuiBackendFlags_PlatformHasViewports`): Backend platform has viewports
        PlatformHasViewports,
        /// Dear ImGui (`ImGuiBackendFlags_HasMouseHoveredViewport`): Backend has mouse hovered viewport
        HasMouseHoveredViewport,
        /// Dear ImGui (`ImGuiBackendFlags_HasParentViewport`): Backend has parent viewport
        HasParentViewport,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiTableFlags`): Flags for `BeginTable()`
    pub TableFlags: ImGuiTableFlags_ {
        /// Dear ImGui (`ImGuiTableFlags_None`): No flags
        None,

        // Features
        /// Dear ImGui (`ImGuiTableFlags_Resizable`): Enable resizing columns.
        Resizable,
        /// Dear ImGui (`ImGuiTableFlags_Reorderable`): Enable reordering columns in header row.
        Reorderable,
        /// Dear ImGui (`ImGuiTableFlags_Hideable`): Enable hiding/disabling columns in context menu.
        Hideable,
        /// Dear ImGui (`ImGuiTableFlags_Sortable`): Enable sorting. Call TableGetSortSpecs() to obtain sort specs.
        Sortable,
        /// Dear ImGui (`ImGuiTableFlags_NoSavedSettings`): Disable persisting columns order, width, visibility and sort settings in the .ini file.
        NoSavedSettings,
        /// Dear ImGui (`ImGuiTableFlags_ContextMenuInBody`): Right-click on columns body/contents will also display table context menu.
        ContextMenuInBody,
        /// Dear ImGui (`ImGuiTableFlags_RowBg`): Set each RowBg color with ImGuiCol_TableRowBg or ImGuiCol_TableRowBgAlt
        RowBg,
        /// Dear ImGui (`ImGuiTableFlags_BordersInnerH`): Draw horizontal borders between rows.
        BordersInnerH,
        /// Dear ImGui (`ImGuiTableFlags_BordersOuterH`): Draw horizontal borders at the top and bottom.
        BordersOuterH,
        /// Dear ImGui (`ImGuiTableFlags_BordersInnerV`): Draw vertical borders between columns.
        BordersInnerV,
        /// Dear ImGui (`ImGuiTableFlags_BordersOuterV`): Draw vertical borders on the left and right sides.
        BordersOuterV,
        /// Dear ImGui (`ImGuiTableFlags_BordersH`): Draw horizontal borders.
        BordersH,
        /// Dear ImGui (`ImGuiTableFlags_BordersV`): Draw vertical borders.
        BordersV,
        /// Dear ImGui (`ImGuiTableFlags_BordersInner`): Draw inner borders.
        BordersInner,
        /// Dear ImGui (`ImGuiTableFlags_BordersOuter`): Draw outer borders.
        BordersOuter,
        /// Dear ImGui (`ImGuiTableFlags_Borders`): Draw all borders.
        Borders,
        /// Dear ImGui (`ImGuiTableFlags_NoBordersInBody`): [ALPHA] Disable vertical borders in columns Body (borders will always appear in Headers).
        NoBordersInBody,
        /// Dear ImGui (`ImGuiTableFlags_NoBordersInBodyUntilResize`): [ALPHA] Disable vertical borders in columns Body until hovered for resize (borders will always appear in Headers).
        NoBordersInBodyUntilResize,
        /// Dear ImGui (`ImGuiTableFlags_SizingFixedFit`): Columns default to _WidthFixed or _WidthAuto, matching contents width.
        SizingFixedFit,
        /// Dear ImGui (`ImGuiTableFlags_SizingFixedSame`): Columns default to _WidthFixed or _WidthAuto, matching the maximum contents width of all columns.
        SizingFixedSame,
        /// Dear ImGui (`ImGuiTableFlags_SizingStretchProp`): Columns default to _WidthStretch with default weights proportional to each columns contents widths.
        SizingStretchProp,
        /// Dear ImGui (`ImGuiTableFlags_SizingStretchSame`): Columns default to _WidthStretch with default weights all equal.
        SizingStretchSame,
        /// Dear ImGui (`ImGuiTableFlags_NoHostExtendX`): Make outer width auto-fit to columns, overriding outer_size.x value.
        NoHostExtendX,
        /// Dear ImGui (`ImGuiTableFlags_NoHostExtendY`): Make outer height stop exactly at outer_size.y (prevent auto-extending table past the limit).
        NoHostExtendY,
        /// Dear ImGui (`ImGuiTableFlags_NoKeepColumnsVisible`): Disable keeping column always minimally visible when ScrollX is off and table gets too small.
        NoKeepColumnsVisible,
        /// Dear ImGui (`ImGuiTableFlags_PreciseWidths`): Disable distributing remainder width to stretched columns.
        PreciseWidths,
        /// Dear ImGui (`ImGuiTableFlags_NoClip`): No clip
        NoClip,
        /// Dear ImGui (`ImGuiTableFlags_PadOuterX`): Pad outer X
        PadOuterX,
        /// Dear ImGui (`ImGuiTableFlags_NoPadOuterX`): No pad outer X
        NoPadOuterX,
        /// Dear ImGui (`ImGuiTableFlags_NoPadInnerX`): No pad inner X
        NoPadInnerX,
        /// Dear ImGui (`ImGuiTableFlags_ScrollX`): Scroll X
        ScrollX,
        /// Dear ImGui (`ImGuiTableFlags_ScrollY`): Scroll Y
        ScrollY,
        /// Dear ImGui (`ImGuiTableFlags_SortMulti`): Sort multi
        SortMulti,
        /// Dear ImGui (`ImGuiTableFlags_SortTristate`): Sort tristate
        SortTristate,
        /// Dear ImGui (`ImGuiTableFlags_HighlightHoveredColumn`): Highlight hovered column
        HighlightHoveredColumn,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiTableRowFlags`): Flags for `TableNextRow()`
    pub TableRowFlags: ImGuiTableRowFlags_ {
        /// Dear ImGui (`ImGuiTableRowFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiTableRowFlags_Headers`): Row is a header
        Headers,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiTableColumnFlags`): Flags for `TableSetupColumn()`
    pub TableColumnFlags: ImGuiTableColumnFlags_ {
        /// Dear ImGui (`ImGuiTableColumnFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiTableColumnFlags_Disabled`): Overriding/master disable flag: hide column, won't show in context menu.
        Disabled,
        /// Dear ImGui (`ImGuiTableColumnFlags_DefaultHide`): Default as a hidden/disabled column.
        DefaultHide,
        /// Dear ImGui (`ImGuiTableColumnFlags_DefaultSort`): Default as a sorting column.
        DefaultSort,
        /// Dear ImGui (`ImGuiTableColumnFlags_WidthStretch`): Column will stretch.
        WidthStretch,
        /// Dear ImGui (`ImGuiTableColumnFlags_WidthFixed`): Column will not stretch.
        WidthFixed,
        /// Dear ImGui (`ImGuiTableColumnFlags_NoResize`): Disable manual resizing.
        NoResize,
        /// Dear ImGui (`ImGuiTableColumnFlags_NoReorder`): Disable manual reordering this column.
        NoReorder,
        /// Dear ImGui (`ImGuiTableColumnFlags_NoHide`): Disable ability to hide/disable this column.
        NoHide,
        /// Dear ImGui (`ImGuiTableColumnFlags_NoClip`): Disable clipping for this column.
        NoClip,
        /// Dear ImGui (`ImGuiTableColumnFlags_NoSort`): Disable ability to sort on this field.
        NoSort,
        /// Dear ImGui (`ImGuiTableColumnFlags_NoSortAscending`): Disable ability to sort in the ascending direction.
        NoSortAscending,
        /// Dear ImGui (`ImGuiTableColumnFlags_NoSortDescending`): Disable ability to sort in the descending direction.
        NoSortDescending,
        /// Dear ImGui (`ImGuiTableColumnFlags_NoHeaderLabel`): TableHeadersRow() will submit an empty label for this column.
        NoHeaderLabel,
        /// Dear ImGui (`ImGuiTableColumnFlags_NoHeaderWidth`): Disable header text width contribution to automatic column width.
        NoHeaderWidth,
        /// Dear ImGui (`ImGuiTableColumnFlags_PreferSortAscending`): Make the initial sort direction Ascending when first sorting on this column.
        PreferSortAscending,
        /// Dear ImGui (`ImGuiTableColumnFlags_PreferSortDescending`): Make the initial sort direction Descending when first sorting on this column.
        PreferSortDescending,
        /// Dear ImGui (`ImGuiTableColumnFlags_IndentEnable`): Use current Indent value when entering cell.
        IndentEnable,
        /// Dear ImGui (`ImGuiTableColumnFlags_IndentDisable`): Ignore current Indent value when entering cell.
        IndentDisable,
        /// Dear ImGui (`ImGuiTableColumnFlags_AngledHeader`): TableHeadersRow() will submit an angled header row for this column.
        AngledHeader,
        /// Dear ImGui (`ImGuiTableColumnFlags_IsEnabled`): Status: is enabled == not hidden by user/api.
        IsEnabled,
        /// Dear ImGui (`ImGuiTableColumnFlags_IsVisible`): Status: is visible == is enabled AND not clipped by scrolling.
        IsVisible,
        /// Dear ImGui (`ImGuiTableColumnFlags_IsSorted`): Status: is currently part of the sort specs.
        IsSorted,
        /// Dear ImGui (`ImGuiTableColumnFlags_IsHovered`): Status: is hovered by mouse.
        IsHovered,
    }
}

imgui_enum! {
    /// Dear ImGui (`ImGuiTableBgTarget`): Color target for `TableSetBgColor()`
    pub TableBgTarget: ImGuiTableBgTarget_ {
        /// Dear ImGui (`ImGuiTableBgTarget_None`): None
        None,
        /// Dear ImGui (`ImGuiTableBgTarget_RowBg0`): Row background 0
        RowBg0,
        /// Dear ImGui (`ImGuiTableBgTarget_RowBg1`): Row background 1
        RowBg1,
        /// Dear ImGui (`ImGuiTableBgTarget_CellBg`): Cell background
        CellBg,
    }
}

imgui_flags_ex! {
    /// Dear ImGui (`ImGuiDockNodeFlags`): Flags for `DockSpace()`
    pub DockNodeFlags: ImGuiDockNodeFlags_ {
        /// Dear ImGui (`ImGuiDockNodeFlags_None`): No flags
        None = ImGuiDockNodeFlags_None,
        /// Dear ImGui (`ImGuiDockNodeFlags_KeepAliveOnly`): Keep alive only
        KeepAliveOnly = ImGuiDockNodeFlags_KeepAliveOnly,
        /// Dear ImGui (`ImGuiDockNodeFlags_NoDockingOverCentralNode`): No docking over central node
        NoDockingOverCentralNode = ImGuiDockNodeFlags_NoDockingOverCentralNode,
        /// Dear ImGui (`ImGuiDockNodeFlags_PassthruCentralNode`): Passthru central node
        PassthruCentralNode = ImGuiDockNodeFlags_PassthruCentralNode,
        /// Dear ImGui (`ImGuiDockNodeFlags_NoDockingSplit`): No docking split
        NoDockingSplit = ImGuiDockNodeFlags_NoDockingSplit,
        /// Dear ImGui (`ImGuiDockNodeFlags_NoResize`): No resize
        NoResize = ImGuiDockNodeFlags_NoResize,
        /// Dear ImGui (`ImGuiDockNodeFlags_AutoHideTabBar`): Auto-hide tab bar
        AutoHideTabBar = ImGuiDockNodeFlags_AutoHideTabBar,
        /// Dear ImGui (`ImGuiDockNodeFlags_NoUndocking`): No undocking
        NoUndocking = ImGuiDockNodeFlags_NoUndocking,
        /// Dear ImGui (`ImGuiDockNodeFlags_DockSpace`): Internal: DockSpace
        DockSpace = ImGuiDockNodeFlagsPrivate_::ImGuiDockNodeFlags_DockSpace,
        /// Dear ImGui (`ImGuiDockNodeFlags_CentralNode`): Internal: Central node
        CentralNode = ImGuiDockNodeFlagsPrivate_::ImGuiDockNodeFlags_CentralNode,
        /// Dear ImGui (`ImGuiDockNodeFlags_NoTabBar`): Internal: No tab bar
        NoTabBar = ImGuiDockNodeFlagsPrivate_::ImGuiDockNodeFlags_NoTabBar,
        /// Dear ImGui (`ImGuiDockNodeFlags_HiddenTabBar`): Internal: Hidden tab bar
        HiddenTabBar = ImGuiDockNodeFlagsPrivate_::ImGuiDockNodeFlags_HiddenTabBar,
        /// Dear ImGui (`ImGuiDockNodeFlags_NoWindowMenuButton`): Internal: No window menu button
        NoWindowMenuButton = ImGuiDockNodeFlagsPrivate_::ImGuiDockNodeFlags_NoWindowMenuButton,
        /// Dear ImGui (`ImGuiDockNodeFlags_NoCloseButton`): Internal: No close button
        NoCloseButton = ImGuiDockNodeFlagsPrivate_::ImGuiDockNodeFlags_NoCloseButton,
        /// Dear ImGui (`ImGuiDockNodeFlags_NoResizeX`): Internal: No resize X
        NoResizeX = ImGuiDockNodeFlagsPrivate_::ImGuiDockNodeFlags_NoResizeX,
        /// Dear ImGui (`ImGuiDockNodeFlags_NoResizeY`): Internal: No resize Y
        NoResizeY = ImGuiDockNodeFlagsPrivate_::ImGuiDockNodeFlags_NoResizeY,
        /// Dear ImGui (`ImGuiDockNodeFlags_DockedWindowsInFocusRoute`): Internal: Docked windows in focus route
        DockedWindowsInFocusRoute = ImGuiDockNodeFlagsPrivate_::ImGuiDockNodeFlags_DockedWindowsInFocusRoute,
        /// Dear ImGui (`ImGuiDockNodeFlags_NoDockingSplitOther`): Internal: No docking split other
        NoDockingSplitOther = ImGuiDockNodeFlagsPrivate_::ImGuiDockNodeFlags_NoDockingSplitOther,
        /// Dear ImGui (`ImGuiDockNodeFlags_NoDockingOverMe`): Internal: No docking over me
        NoDockingOverMe = ImGuiDockNodeFlagsPrivate_::ImGuiDockNodeFlags_NoDockingOverMe,
        /// Dear ImGui (`ImGuiDockNodeFlags_NoDockingOverOther`): Internal: No docking over other
        NoDockingOverOther = ImGuiDockNodeFlagsPrivate_::ImGuiDockNodeFlags_NoDockingOverOther,
        /// Dear ImGui (`ImGuiDockNodeFlags_NoDockingOverEmpty`): Internal: No docking over empty
        NoDockingOverEmpty = ImGuiDockNodeFlagsPrivate_::ImGuiDockNodeFlags_NoDockingOverEmpty,
        /// Dear ImGui (`ImGuiDockNodeFlags_NoDocking`): Internal: No docking
        NoDocking = ImGuiDockNodeFlagsPrivate_::ImGuiDockNodeFlags_NoDocking,
    }
}

// ImGuiDragDropFlags is split into two bitflags, one for the Source, one for the Accept.
imgui_flags_ex! {
    /// Dear ImGui (`ImGuiDragDropFlags`): Flags for `BeginDragDropSource()`
    pub DragDropSourceFlags: ImGuiDragDropFlags_ {
        /// Dear ImGui (`ImGuiDragDropFlags_None`): No flags
        None = ImGuiDragDropFlags_None,
        /// Dear ImGui (`ImGuiDragDropFlags_SourceNoPreviewTooltip`): No preview tooltip
        NoPreviewTooltip = ImGuiDragDropFlags_SourceNoPreviewTooltip,
        /// Dear ImGui (`ImGuiDragDropFlags_SourceNoDisableHover`): No disable hover
        NoDisableHover = ImGuiDragDropFlags_SourceNoDisableHover,
        /// Dear ImGui (`ImGuiDragDropFlags_SourceNoHoldToOpenOthers`): No hold to open others
        NoHoldToOpenOthers = ImGuiDragDropFlags_SourceNoHoldToOpenOthers,
        /// Dear ImGui (`ImGuiDragDropFlags_SourceAllowNullID`): Allow null ID
        AllowNullID = ImGuiDragDropFlags_SourceAllowNullID,
        /// Dear ImGui (`ImGuiDragDropFlags_SourceExtern`): Extern
        Extern = ImGuiDragDropFlags_SourceExtern,
        /// Dear ImGui (`ImGuiDragDropFlags_PayloadAutoExpire`): Payload auto expire
        PayloadAutoExpire = ImGuiDragDropFlags_PayloadAutoExpire,
        /// Dear ImGui (`ImGuiDragDropFlags_PayloadNoCrossContext`): Payload no cross context
        PayloadNoCrossContext = ImGuiDragDropFlags_PayloadNoCrossContext,
        /// Dear ImGui (`ImGuiDragDropFlags_PayloadNoCrossProcess`): Payload no cross process
        PayloadNoCrossProcess = ImGuiDragDropFlags_PayloadNoCrossProcess,
    }
}
imgui_flags_ex! {
    /// Dear ImGui (`ImGuiDragDropFlags`): Flags for `AcceptDragDropPayload()`
    pub DragDropAcceptFlags: ImGuiDragDropFlags_ {
        /// Dear ImGui (`ImGuiDragDropFlags_None`): No flags
        None = ImGuiDragDropFlags_None,
        /// Dear ImGui (`ImGuiDragDropFlags_AcceptBeforeDelivery`): Accept before delivery
        BeforeDelivery = ImGuiDragDropFlags_AcceptBeforeDelivery,
        /// Dear ImGui (`ImGuiDragDropFlags_AcceptNoDrawDefaultRect`): No draw default rect
        NoDrawDefaultRect = ImGuiDragDropFlags_AcceptNoDrawDefaultRect,
        /// Dear ImGui (`ImGuiDragDropFlags_AcceptNoPreviewTooltip`): No preview tooltip
        NoPreviewTooltip = ImGuiDragDropFlags_AcceptNoPreviewTooltip,
        /// Dear ImGui (`ImGuiDragDropFlags_AcceptDrawAsHovered`): Accept draw as hovered
        AcceptDrawAsHovered = ImGuiDragDropFlags_AcceptDrawAsHovered,
        /// Dear ImGui (`ImGuiDragDropFlags_AcceptPeekOnly`): Accept peek only
        PeekOnly = ImGuiDragDropFlags_AcceptPeekOnly,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiInputFlags`): Flags for `Shortcut()`, `SetNextItemShortcut()`
    pub InputFlags: ImGuiInputFlags_ {
        /// Dear ImGui (`ImGuiInputFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiInputFlags_Repeat`): Repeat
        Repeat,
        /// Dear ImGui (`ImGuiInputFlags_RouteActive`): Route active
        RouteActive,
        /// Dear ImGui (`ImGuiInputFlags_RouteFocused`): Route focused
        RouteFocused,
        /// Dear ImGui (`ImGuiInputFlags_RouteGlobal`): Route global
        RouteGlobal,
        /// Dear ImGui (`ImGuiInputFlags_RouteAlways`): Route always
        RouteAlways,
        /// Dear ImGui (`ImGuiInputFlags_RouteOverFocused`): Route over focused
        RouteOverFocused,
        /// Dear ImGui (`ImGuiInputFlags_RouteOverActive`): Route over active
        RouteOverActive,
        /// Dear ImGui (`ImGuiInputFlags_RouteUnlessBgFocused`): Route unless bg focused
        RouteUnlessBgFocused,
        /// Dear ImGui (`ImGuiInputFlags_RouteFromRootWindow`): Route from root window
        RouteFromRootWindow,
        /// Dear ImGui (`ImGuiInputFlags_Tooltip`): Tooltip
        Tooltip,
    }
}

imgui_scoped_enum! {
    /// Dear ImGui (`ImGuiSortDirection`): Sorting direction (ascending or descending)
    pub SortDirection: ImGuiSortDirection {
        /// Dear ImGui (`ImGuiSortDirection_None`): No direction
        None,
        /// Dear ImGui (`ImGuiSortDirection_Ascending`): Ascending
        Ascending,
        /// Dear ImGui (`ImGuiSortDirection_Descending`): Descending
        Descending,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiItemFlags`): Flags for `PushItemFlag()`, shared by all items
    pub ItemFlags: ImGuiItemFlags_ {
        /// Dear ImGui (`ImGuiItemFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiItemFlags_NoTabStop`): No tab stop
        NoTabStop,
        /// Dear ImGui (`ImGuiItemFlags_NoNav`): No nav
        NoNav,
        /// Dear ImGui (`ImGuiItemFlags_NoNavDefaultFocus`): No nav default focus
        NoNavDefaultFocus,
        /// Dear ImGui (`ImGuiItemFlags_ButtonRepeat`): Button repeat
        ButtonRepeat,
        /// Dear ImGui (`ImGuiItemFlags_AutoClosePopups`): Auto-close popups
        AutoClosePopups,
        /// Dear ImGui (`ImGuiItemFlags_AllowDuplicateId`): Allow duplicate ID
        AllowDuplicateId,
        /// Dear ImGui (`ImGuiItemFlags_LiveEditOnInputText`): InputText: apply keyboard edits to backing value while typing
        LiveEditOnInputText,
        /// Dear ImGui (`ImGuiItemFlags_LiveEditOnInputScalar`): DragXXX, SliderXXX, InputScalar: apply keyboard edits to backing value while typing
        LiveEditOnInputScalar,
        /// Dear ImGui (`ImGuiItemFlags_LiveEditOnInput`): LiveEditOnInputText | LiveEditOnInputScalar
        LiveEditOnInput,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImGuiMultiSelectFlags`): Flags for `BeginMultiSelect()`
    pub MultiSelectFlags: ImGuiMultiSelectFlags_ {
        /// Dear ImGui (`ImGuiMultiSelectFlags_None`): No flags
        None,
        /// Dear ImGui (`ImGuiMultiSelectFlags_SingleSelect`): Single select
        SingleSelect,
        /// Dear ImGui (`ImGuiMultiSelectFlags_NoSelectAll`): No select all
        NoSelectAll,
        /// Dear ImGui (`ImGuiMultiSelectFlags_NoRangeSelect`): No range select
        NoRangeSelect,
        /// Dear ImGui (`ImGuiMultiSelectFlags_NoAutoSelect`): No auto select
        NoAutoSelect,
        /// Dear ImGui (`ImGuiMultiSelectFlags_NoAutoClear`): No auto clear
        NoAutoClear,
        /// Dear ImGui (`ImGuiMultiSelectFlags_NoAutoClearOnReselect`): No auto clear on reselect
        NoAutoClearOnReselect,
        /// Dear ImGui (`ImGuiMultiSelectFlags_BoxSelect1d`): Box select 1d
        BoxSelect1d,
        /// Dear ImGui (`ImGuiMultiSelectFlags_BoxSelect2d`): Box select 2d
        BoxSelect2d,
        /// Dear ImGui (`ImGuiMultiSelectFlags_BoxSelectNoScroll`): Box select no scroll
        BoxSelectNoScroll,
        /// Dear ImGui (`ImGuiMultiSelectFlags_ClearOnEscape`): Clear on escape
        ClearOnEscape,
        /// Dear ImGui (`ImGuiMultiSelectFlags_ClearOnClickVoid`): Clear on click void
        ClearOnClickVoid,
        /// Dear ImGui (`ImGuiMultiSelectFlags_ScopeWindow`): Scope window
        ScopeWindow,
        /// Dear ImGui (`ImGuiMultiSelectFlags_ScopeRect`): Scope rect
        ScopeRect,
        /// Dear ImGui (`ImGuiMultiSelectFlags_SelectOnAuto`): Select on auto
        SelectOnAuto,
        /// Dear ImGui (`ImGuiMultiSelectFlags_SelectOnClickAlways`): Select on click always
        SelectOnClickAlways,
        /// Dear ImGui (`ImGuiMultiSelectFlags_SelectOnClickRelease`): Select on click release
        SelectOnClickRelease,
        //RangeSelect2d,
        /// Dear ImGui (`ImGuiMultiSelectFlags_NavWrapX`): Nav wrap X
        NavWrapX,
        /// Dear ImGui (`ImGuiMultiSelectFlags_NoSelectOnRightClick`): No select on right click
        NoSelectOnRightClick,
    }
}

imgui_scoped_enum! {
    /// Dear ImGui (`ImGuiSelectionRequestType`): Selection request type
    pub SelectionRequestType: ImGuiSelectionRequestType {
        /// Dear ImGui (`ImGuiSelectionRequestType_None`): None
        None,
        /// Dear ImGui (`ImGuiSelectionRequestType_SetAll`): Request app to clear or select all
        SetAll,
        /// Dear ImGui (`ImGuiSelectionRequestType_SetRange`): Request app to select/unselect range
        SetRange,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImFontAtlasFlags`): Flags for `ImFontAtlas`
    pub FontAtlasFlags: ImFontAtlasFlags_ {
        /// Dear ImGui (`ImFontAtlasFlags_None`): No flags
        None,
        /// Dear ImGui (`ImFontAtlasFlags_NoPowerOfTwoHeight`): No power of two height
        NoPowerOfTwoHeight,
        /// Dear ImGui (`ImFontAtlasFlags_NoMouseCursors`): No mouse cursors
        NoMouseCursors,
        /// Dear ImGui (`ImFontAtlasFlags_NoBakedLines`): No baked lines
        NoBakedLines,
    }
}

imgui_flags! {
    /// Dear ImGui (`ImFontFlags`): Flags for `ImFont`
    pub FontFlags: ImFontFlags_ {
        /// Dear ImGui (`ImFontFlags_None`): No flags
        None,
        /// Dear ImGui (`ImFontFlags_NoLoadError`): No load error
        NoLoadError,

        // internal but bound anyways
        /// Dear ImGui (`ImFontFlags_NoLoadGlyphs`): No load glyphs
        NoLoadGlyphs,
        /// Dear ImGui (`ImFontFlags_LockBakedSizes`): Lock baked sizes
        LockBakedSizes,
    }
}
