struct ImGuiContext;
thread_local ImGuiContext* MyImGuiTLS;

#define GImGui MyImGuiTLS

#include "imgui/imgui.cpp"
#include "imgui/imgui_widgets.cpp"
#include "imgui/imgui_draw.cpp"
#include "imgui/imgui_tables.cpp"
#include "imgui/imgui_demo.cpp"
#ifdef IMGUI_ENABLE_FREETYPE
    #include "imgui/misc/freetype/imgui_freetype.cpp"
#endif
