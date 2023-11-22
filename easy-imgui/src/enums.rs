#![allow(non_upper_case_globals)]
use easy_imgui_sys::*;

macro_rules! imgui_enum_ex {
    ($vis:vis $name:ident: $native_name:ident { $($field:ident = $value:ident),* $(,)? }) => {
        #[repr(i32)]
        #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
        $vis enum $name {
            $(
                $field = $native_name::$value.0 as i32,
            )*
        }
        impl $name {
            pub fn bits(self) -> i32 {
                self as i32
            }
            pub fn from_bits(bits: i32) -> Option<Self> {
                $(
                    const $field: i32 = $native_name::$value.0 as i32;
                )*
                let r = match bits {
                    $(
                        $field => Self::$field,
                    )*
                    _ => return std::option::Option::None,
                };
                Some(r)
            }
        }
    }
}

macro_rules! imgui_enum {
    ($vis:vis $name:ident: $native_name:ident { $($field:ident ),* $(,)? }) => {
        paste::paste! {
            imgui_enum_ex! {
                $vis $name: $native_name {
                    $($field = [<$native_name $field>],)*
                }
            }
        }
    };
}

macro_rules! imgui_flags_ex {
    ($vis:vis $name:ident: $native_name:ident { $($field:ident = $value:ident),* $(,)? }) => {
        bitflags::bitflags! {
            $vis struct $name : i32 {
                $(
                    const $field = $native_name::$value.0 as i32;
                )*
            }
        }
};
}

macro_rules! imgui_flags {
    ($vis:vis $name:ident: $native_name:ident { $($field:ident),* $(,)? }) => {
        paste::paste! {
            imgui_flags_ex! {
                $vis $name: $native_name {
                    $($field = [<$native_name $field>],)*
                }
            }
        }
    };
}
imgui_flags!{
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

imgui_enum!{
    pub Cond: ImGuiCond_ {
        Always,
        Once,
        FirstUseEver,
        Appearing,
    }
}

imgui_enum!{
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

imgui_enum!{
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
        ButtonTextAlign,
        SelectableTextAlign,
        SeparatorTextBorderSize,
        SeparatorTextAlign,
        SeparatorTextPadding,
    }
}

imgui_flags!{
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
        NoNav,
        NoDecoration,
        NoInputs,
    }
}

imgui_flags!{
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
imgui_flags!{
    pub ButtonFlags: ImGuiButtonFlags_ {
        None,
        MouseButtonLeft,
        MouseButtonRight,
        MouseButtonMiddle,
    }
}

imgui_enum!{
    pub Dir: ImGuiDir_ {
        Left,
        Right,
        Up,
        Down,
    }
}

imgui_flags!{
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

imgui_flags!{
    pub SelectableFlags: ImGuiSelectableFlags_ {
        None,
        DontClosePopups,
        SpanAllColumns,
        AllowDoubleClick,
        Disabled,
        AllowOverlap,
    }
}

imgui_flags!{
    pub SliderFlags: ImGuiSliderFlags_ {
        None,
        AlwaysClamp,
        Logarithmic,
        NoRoundToFormat,
        NoInput,
    }
}

imgui_flags!{
    pub InputTextFlags: ImGuiInputTextFlags_ {
        None,
        CharsDecimal,
        CharsHexadecimal,
        CharsUppercase,
        CharsNoBlank,
        AutoSelectAll,
        EnterReturnsTrue,
        CallbackCompletion,
        CallbackHistory,
        CallbackAlways,
        CallbackCharFilter,
        AllowTabInput,
        CtrlEnterForNewLine,
        NoHorizontalScroll,
        AlwaysOverwrite,
        ReadOnly,
        Password,
        NoUndoRedo,
        CharsScientific,
        CallbackResize,
        CallbackEdit,
        EscapeClearsAll,
    }
}

imgui_flags!{
    pub HoveredFlags: ImGuiHoveredFlags_ {
        None,
        ChildWindows,
        RootWindow,
        AnyWindow,
        NoPopupHierarchy,
        //DockHierarchy,
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


// MouseButton is hand-crafted.

pub const MOUSE_BUTTON_COUNT: u32 = ImGuiMouseButton_::ImGuiMouseButton_COUNT.0;

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
            MouseButton::Right =>  ImGuiMouseButton_::ImGuiMouseButton_Right.0 as i32,
            MouseButton::Middle =>  ImGuiMouseButton_::ImGuiMouseButton_Middle.0 as i32,
            MouseButton::Other(x) => x as i32
        }
    }
}

imgui_enum!{
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
imgui_enum_ex!{
    pub Key: ImGuiKey {
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

        // Modifiers
        ModCtrl = ImGuiMod_Ctrl,
        ModShift = ImGuiMod_Shift,
        ModAlt = ImGuiMod_Alt,
        ModSuper = ImGuiMod_Super,
    }
}

imgui_flags!{
    pub ViewportFlags: ImGuiViewportFlags_ {
        None,
        IsPlatformWindow,
        IsPlatformMonitor,
        OwnedByApp,
    }
}

imgui_flags!{
    pub PopupFlags: ImGuiPopupFlags_ {
        None,
        MouseButtonLeft,
        MouseButtonRight,
        MouseButtonMiddle,
        MouseButtonMask_,
        MouseButtonDefault_,
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
        //DockHierarchy,
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