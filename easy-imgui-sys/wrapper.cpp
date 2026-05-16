struct ImGuiContext;
thread_local ImGuiContext* MyImGuiTLS;

#include "imgui.h"
#include "non-inlines.cpp"

#ifdef _MSC_VER
#include "hack_msvc.cpp"
#endif
