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
                self.0
            }
            #[inline]
            fn from_bits(t: c_int) -> Self {
                Self(t)
            }
        }
    };
}

macro_rules! imgui_enum_ex {
    ($vis:vis $name:ident : $native_name:ident : $native_name_api:ty { $( $(#[$inner:ident $($args:tt)*])* $field:ident = $value:ident),* $(,)? }) => {
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
    ($vis:vis $name:ident: $native_name:ident { $( $(#[$inner:ident $($args:tt)*])* $field:ident ),* $(,)? }) => {
        paste::paste! {
            imgui_enum_ex! {
                $vis $name: $native_name: i32 {
                    $( $(#[$inner $($args)*])* $field = [<$native_name $field>],)*
                }
            }
        }
    };
}

// Just like imgui_enum but for native strong C++ enums
macro_rules! imgui_scoped_enum {
    ($vis:vis $name:ident: $native_name:ident { $( $(#[$inner:ident $($args:tt)*])* $field:ident ),* $(,)? }) => {
        impl_bit_enum_helper!{$native_name}
        paste::paste! {
            imgui_enum_ex! {
                $vis $name: $native_name: $native_name {
                    $( $(#[$inner $($args)*])* $field = [<$native_name _ $field>],)*
                }
            }
        }
    };
}

macro_rules! imgui_flags_ex {
    ($vis:vis $name:ident: $native_name:ident { $( $(#[$inner:ident $($args:tt)*])* $field:ident = $value:ident),* $(,)? }) => {
        bitflags::bitflags! {
            #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
            $vis struct $name : i32 {
                $(
                    $(#[$inner $($args)*])*
                    const $field = $native_name::$value.0 as i32;
                )*
            }
        }
};
}

macro_rules! imgui_flags {
    ($vis:vis $name:ident: $native_name:ident { $( $(#[$inner:ident $($args:tt)*])* $field:ident),* $(,)? }) => {
        paste::paste! {
            imgui_flags_ex! {
                $vis $name: $native_name {
                    $( $(#[$inner $($args)*])* $field = [<$native_name $field>],)*
                }
            }
        }
    };
}
imgui_flags! {
    pub DrawFlags: ImDrawFlags_ {
        None,
        Closed,
        RoundCornersTopLeft,
        RoundCornersTopRight,
        RoundCornersBottomLeft,
        RoundCornersBottomRight,
        RoundCornersNone,
        RoundCornersTop,
        RoundCornersBottom,
        RoundCornersLeft,
        RoundCornersRight,
        RoundCornersAll,
    }
}

imgui_enum! {
    pub Cond: ImGuiCond_ {
        Always,
        Once,
        FirstUseEver,
        Appearing,
    }
}

imgui_enum! {
    pub ColorId: ImGuiCol_ {
        Text,
        TextDisabled,
        WindowBg,
        ChildBg,
        PopupBg,
        Border,
        BorderShadow,
        FrameBg,
        FrameBgHovered,
        FrameBgActive,
        TitleBg,
        TitleBgActive,
        TitleBgCollapsed,
        MenuBarBg,
        ScrollbarBg,
        ScrollbarGrab,
        ScrollbarGrabHovered,
        ScrollbarGrabActive,
        CheckMark,
        SliderGrab,
        SliderGrabActive,
        Button,
        ButtonHovered,
        ButtonActive,
        Header,
        HeaderHovered,
        HeaderActive,
        Separator,
        SeparatorHovered,
        SeparatorActive,
        ResizeGrip,
        ResizeGripHovered,
        ResizeGripActive,
        Tab,
        TabHovered,
        TabActive,
        TabUnfocused,
        TabUnfocusedActive,
        #[cfg(feature="docking")]
        DockingPreview,
        #[cfg(feature="docking")]
        DockingEmptyBg,
        PlotLines,
        PlotLinesHovered,
        PlotHistogram,
        PlotHistogramHovered,
        TableHeaderBg,
        TableBorderStrong,
        TableBorderLight,
        TableRowBg,
        TableRowBgAlt,
        TextSelectedBg,
        DragDropTarget,
        NavHighlight,
        NavWindowingHighlight,
        NavWindowingDimBg,
        ModalWindowDimBg,
    }
}

imgui_enum! {
    pub StyleVar: ImGuiStyleVar_ {
        Alpha,
        DisabledAlpha,
        WindowPadding,
        WindowRounding,
        WindowBorderSize,
        WindowMinSize,
        WindowTitleAlign,
        ChildRounding,
        ChildBorderSize,
        PopupRounding,
        PopupBorderSize,
        FramePadding,
        FrameRounding,
        FrameBorderSize,
        ItemSpacing,
        ItemInnerSpacing,
        IndentSpacing,
        CellPadding,
        ScrollbarSize,
        ScrollbarRounding,
        GrabMinSize,
        GrabRounding,
        TabRounding,
        TabBorderSize,
        TabBarBorderSize,
        TableAngledHeadersAngle,
        TableAngledHeadersTextAlign,
        ButtonTextAlign,
        SelectableTextAlign,
        SeparatorTextBorderSize,
        SeparatorTextAlign,
        SeparatorTextPadding,
        #[cfg(feature="docking")]
        DockingSeparatorSize,
    }
}

imgui_flags! {
    pub WindowFlags: ImGuiWindowFlags_ {
        None,
        NoTitleBar,
        NoResize,
        NoMove,
        NoScrollbar,
        NoScrollWithMouse,
        NoCollapse,
        AlwaysAutoResize,
        NoBackground,
        NoSavedSettings,
        NoMouseInputs,
        MenuBar,
        HorizontalScrollbar,
        NoFocusOnAppearing,
        NoBringToFrontOnFocus,
        AlwaysVerticalScrollbar,
        AlwaysHorizontalScrollbar,
        NoNavInputs,
        NoNavFocus,
        UnsavedDocument,
        #[cfg(feature="docking")]
        NoDocking,
        NoNav,
        NoDecoration,
        NoInputs,
    }
}

imgui_flags! {
    pub ChildFlags: ImGuiChildFlags_ {
        None,
        Border,
        AlwaysUseWindowPadding,
        ResizeX,
        ResizeY,
        AutoResizeX,
        AutoResizeY,
        AlwaysAutoResize,
        FrameStyle,
    }
}
imgui_flags! {
    pub ButtonFlags: ImGuiButtonFlags_ {
        None,
        MouseButtonLeft,
        MouseButtonRight,
        MouseButtonMiddle,
    }
}

imgui_scoped_enum! {
    pub Dir: ImGuiDir {
        Left,
        Right,
        Up,
        Down,
    }
}

imgui_flags! {
    pub ComboFlags: ImGuiComboFlags_ {
        None,
        PopupAlignLeft,
        HeightSmall,
        HeightRegular,
        HeightLarge,
        HeightLargest,
        NoArrowButton,
        NoPreview,
    }
}

imgui_flags! {
    pub SelectableFlags: ImGuiSelectableFlags_ {
        None,
        DontClosePopups,
        SpanAllColumns,
        AllowDoubleClick,
        Disabled,
        AllowOverlap,
    }
}

imgui_flags! {
    pub SliderFlags: ImGuiSliderFlags_ {
        None,
        AlwaysClamp,
        Logarithmic,
        NoRoundToFormat,
        NoInput,
    }
}

imgui_flags! {
    pub InputTextFlags: ImGuiInputTextFlags_ {
        //Basic filters
        None,
        CharsDecimal,
        CharsHexadecimal,
        CharsScientific,
        CharsUppercase,
        CharsNoBlank,

        // Inputs
        AllowTabInput,
        EnterReturnsTrue,
        EscapeClearsAll,
        CtrlEnterForNewLine,

        // Other options
        ReadOnly,
        Password,
        AlwaysOverwrite,
        AutoSelectAll,
        ParseEmptyRefVal,
        DisplayEmptyRefVal,
        NoHorizontalScroll,
        NoUndoRedo,

        // Callback features
        CallbackCompletion,
        CallbackHistory,
        CallbackAlways,
        CallbackCharFilter,
        CallbackResize,
        CallbackEdit,
    }
}

imgui_flags! {
    pub HoveredFlags: ImGuiHoveredFlags_ {
        None,
        ChildWindows,
        RootWindow,
        AnyWindow,
        NoPopupHierarchy,
        #[cfg(feature="docking")]
        DockHierarchy,
        AllowWhenBlockedByPopup,
        //AllowWhenBlockedByModal,
        AllowWhenBlockedByActiveItem,
        AllowWhenOverlappedByItem,
        AllowWhenOverlappedByWindow,
        AllowWhenDisabled,
        NoNavOverride,
        AllowWhenOverlapped,
        RectOnly,
        RootAndChildWindows,
        ForTooltip,
        Stationary,
        DelayNone,
        DelayShort,
        DelayNormal,
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
    pub MouseCursor : ImGuiMouseCursor_ {
        None,
        Arrow,
        TextInput,
        ResizeAll,
        ResizeNS,
        ResizeEW,
        ResizeNESW,
        ResizeNWSE,
        Hand,
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

imgui_flags! {
    pub ViewportFlags: ImGuiViewportFlags_ {
        None,
        IsPlatformWindow,
        IsPlatformMonitor,
        OwnedByApp,
        #[cfg(feature="docking")]
        NoDecoration,
        #[cfg(feature="docking")]
        NoTaskBarIcon,
        #[cfg(feature="docking")]
        NoFocusOnAppearing,
        #[cfg(feature="docking")]
        NoFocusOnClick,
        #[cfg(feature="docking")]
        NoInputs,
        #[cfg(feature="docking")]
        NoRendererClear,
        #[cfg(feature="docking")]
        NoAutoMerge,
        #[cfg(feature="docking")]
        TopMost,
        #[cfg(feature="docking")]
        CanHostOtherWindows,
        #[cfg(feature="docking")]
        IsMinimized,
        #[cfg(feature="docking")]
        IsFocused,
    }
}

imgui_flags! {
    pub PopupFlags: ImGuiPopupFlags_ {
        None,
        MouseButtonLeft,
        MouseButtonRight,
        MouseButtonMiddle,
        MouseButtonMask_,
        MouseButtonDefault_,
        NoReopen,
        NoOpenOverExistingPopup,
        NoOpenOverItems,
        AnyPopupId,
        AnyPopupLevel,
        AnyPopup,
    }
}

imgui_flags! {
    pub ConfigFlags: ImGuiConfigFlags_ {
        None,
        NavEnableKeyboard,
        NavEnableGamepad,
        NavEnableSetMousePos,
        NavNoCaptureKeyboard,
        NoMouse,
        NoMouseCursorChange,
        #[cfg(feature="docking")]
        DockingEnable,
        IsSRGB,
        IsTouchScreen,
    }
}

imgui_flags! {
    pub TreeNodeFlags: ImGuiTreeNodeFlags_ {
        None,
        Selected,
        Framed,
        AllowOverlap,
        NoTreePushOnOpen,
        NoAutoOpenOnLog,
        DefaultOpen,
        OpenOnDoubleClick,
        OpenOnArrow,
        Leaf,
        Bullet,
        FramePadding,
        SpanAvailWidth,
        SpanFullWidth,
        SpanTextWidth,
        SpanAllColumns,
        NavLeftJumpsBackHere,
        //NoScrollOnOpen,
        CollapsingHeader,
    }
}

imgui_flags! {
    pub FocusedFlags: ImGuiFocusedFlags_ {
        None,
        ChildWindows,
        RootWindow,
        AnyWindow,
        NoPopupHierarchy,
        #[cfg(feature="docking")]
        DockHierarchy,
        RootAndChildWindows,
    }
}

imgui_flags! {
    pub ColorEditFlags: ImGuiColorEditFlags_ {
        None,
        NoAlpha,
        NoPicker,
        NoOptions,
        NoSmallPreview,
        NoInputs,
        NoTooltip,
        NoLabel,
        NoSidePreview,
        NoDragDrop,
        NoBorder,
        AlphaBar,
        AlphaPreview,
        AlphaPreviewHalf,
        HDR,
        DisplayRGB,
        DisplayHSV,
        DisplayHex,
        Uint8,
        Float,
        PickerHueBar,
        PickerHueWheel,
        InputRGB,
        InputHSV,
        DefaultOptions_,
    }
}

imgui_flags! {
    pub TabBarFlags: ImGuiTabBarFlags_ {
        None,
        Reorderable,
        AutoSelectNewTabs,
        TabListPopupButton,
        NoCloseWithMiddleMouseButton,
        NoTabListScrollingButtons,
        NoTooltip,
        FittingPolicyResizeDown,
        FittingPolicyScroll,
        FittingPolicyMask_,
        FittingPolicyDefault_,
    }
}

imgui_flags! {
    pub TabItemFlags: ImGuiTabItemFlags_ {
        None,
        UnsavedDocument,
        SetSelected,
        NoCloseWithMiddleMouseButton,
        NoPushId,
        NoTooltip,
        NoReorder,
        Leading,
        Trailing,
    }
}

imgui_flags! {
    pub BackendFlags: ImGuiBackendFlags_ {
        None,
        HasGamepad,
        HasMouseCursors,
        HasSetMousePos,
        RendererHasVtxOffset,
    }
}

imgui_flags! {
    pub TableFlags: ImGuiTableFlags_ {
        None,
        // Features
        Resizable,
        Reorderable,
        Hideable,
        Sortable,
        NoSavedSettings,
        ContextMenuInBody,
        // Decorations
        RowBg,
        BordersInnerH,
        BordersOuterH,
        BordersInnerV,
        BordersOuterV,
        BordersH,
        BordersV,
        BordersInner,
        BordersOuter,
        Borders,
        NoBordersInBody,
        NoBordersInBodyUntilResize,
        // Sizing Policy (read above for defaults)
        SizingFixedFit,
        SizingFixedSame,
        SizingStretchProp,
        SizingStretchSame,
        // Sizing Extra Options
        NoHostExtendX,
        NoHostExtendY,
        NoKeepColumnsVisible,
        PreciseWidths,
        // Clipping
        NoClip,
        // Padding
        PadOuterX,
        NoPadOuterX,
        NoPadInnerX,
        // Scrolling
        ScrollX,
        ScrollY,
        // Sorting
        SortMulti,
        SortTristate,
        // Miscellaneous
        HighlightHoveredColumn,
    }
}

imgui_flags! {
    pub TableRowFlags: ImGuiTableRowFlags_ {
        None,
        Headers,
    }
}

imgui_flags! {
    pub TableColumnFlags: ImGuiTableColumnFlags_ {
        // Input configuration flags
        None,
        Disabled,
        DefaultHide,
        DefaultSort,
        WidthStretch,
        WidthFixed,
        NoResize,
        NoReorder,
        NoHide,
        NoClip,
        NoSort,
        NoSortAscending,
        NoSortDescending,
        NoHeaderLabel,
        NoHeaderWidth,
        PreferSortAscending,
        PreferSortDescending,
        IndentEnable,
        IndentDisable,
        AngledHeader,

        // Output status flags, read-only via Table::get_column_flags()
        IsEnabled,
        IsVisible,
        IsSorted,
        IsHovered,
    }
}

imgui_enum! {
    pub TableBgTarget: ImGuiTableBgTarget_ {
        None,
        RowBg0,
        RowBg1,
        CellBg,
    }
}

#[cfg(feature = "docking")]
imgui_flags! {
    pub DockNodeFlags: ImGuiDockNodeFlags_ {
        None,
        KeepAliveOnly,
        //NoCentralNode,
        NoDockingOverCentralNode,
        PassthruCentralNode,
        NoDockingSplit,
        NoResize,
        AutoHideTabBar,
        NoUndocking,
    }
}

// ImGuiDragDropFlags is split into two bitflags, one for the Source, one for the Accept.
imgui_flags_ex! {
    pub DragDropSourceFlags: ImGuiDragDropFlags_ {
        None = ImGuiDragDropFlags_None,
        NoPreviewTooltip = ImGuiDragDropFlags_SourceNoPreviewTooltip,
        NoDisableHover = ImGuiDragDropFlags_SourceNoDisableHover,
        NoHoldToOpenOthers = ImGuiDragDropFlags_SourceNoHoldToOpenOthers,
        AllowNullID = ImGuiDragDropFlags_SourceAllowNullID,
        Extern = ImGuiDragDropFlags_SourceExtern,
        AutoExpirePayload = ImGuiDragDropFlags_SourceAutoExpirePayload,
    }
}
imgui_flags_ex! {
    pub DragDropAcceptFlags: ImGuiDragDropFlags_ {
        None = ImGuiDragDropFlags_None,
        BeforeDelivery = ImGuiDragDropFlags_AcceptBeforeDelivery,
        NoDrawDefaultRect = ImGuiDragDropFlags_AcceptNoDrawDefaultRect,
        NoPreviewTooltip =  ImGuiDragDropFlags_AcceptNoPreviewTooltip,
        PeekOnly = ImGuiDragDropFlags_AcceptPeekOnly,
    }
}

imgui_flags! {
    pub InputFlags: ImGuiInputFlags_ {
        None,
        RouteActive,
        RouteFocused,
        RouteGlobal,
        RouteAlways,
        RouteOverFocused,
        RouteOverActive,
        RouteUnlessBgFocused,
        RouteFromRootWindow,
        Tooltip,
    }
}
