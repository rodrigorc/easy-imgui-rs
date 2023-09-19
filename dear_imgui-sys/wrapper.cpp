struct ImGuiContext;
thread_local ImGuiContext* MyImGuiTLS;

//#define GImGui MyImGuiTLS

#include "imgui.cpp"
#include "imgui_widgets.cpp"
#include "imgui_draw.cpp"
#include "imgui_tables.cpp"
#include "imgui_demo.cpp"
#ifdef IMGUI_ENABLE_FREETYPE
    #include "misc/freetype/imgui_freetype.cpp"
#endif
