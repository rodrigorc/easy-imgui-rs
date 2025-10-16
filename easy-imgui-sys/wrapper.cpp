struct ImGuiContext;
thread_local ImGuiContext* MyImGuiTLS;

#ifdef _MSC_VER
#include "imgui.h"
#include "hack_msvc.cpp"
#endif