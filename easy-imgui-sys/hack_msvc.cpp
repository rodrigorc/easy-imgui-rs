// Hack for MSVC compilers.
// See the comments in build.rs for the rationale.

// Just like ImVec2 but FFI safe.
struct ImVec2_rr { float x, y; };

// Helper function.
static inline ImVec2_rr _rr(ImVec2 v) { return ImVec2_rr { v.x, v.y }; }

// Required functions, the original ones are into `namespace ImGui` which is nice 
// because their bindgen-names can be matched, and no extra changes are necessary.
ImVec2_rr ImGui_GetWindowPos() { return _rr(ImGui::GetWindowPos()); }
ImVec2_rr ImGui_GetWindowSize() { return _rr(ImGui::GetWindowSize()); }
ImVec2_rr ImGui_GetContentRegionAvail() { return _rr(ImGui::GetContentRegionAvail()); }
ImVec2_rr ImGui_GetFontTexUvWhitePixel() { return _rr(ImGui::GetFontTexUvWhitePixel()); }
ImVec2_rr ImGui_GetCursorScreenPos() { return _rr(ImGui::GetCursorScreenPos()); }
ImVec2_rr ImGui_GetCursorPos() { return _rr(ImGui::GetCursorPos()); }
ImVec2_rr ImGui_GetCursorStartPos() { return _rr(ImGui::GetCursorStartPos()); }
ImVec2_rr ImGui_GetItemRectMin() { return _rr(ImGui::GetItemRectMin()); }
ImVec2_rr ImGui_GetItemRectMax() { return _rr(ImGui::GetItemRectMax()); }
ImVec2_rr ImGui_GetItemRectSize() { return _rr(ImGui::GetItemRectSize()); }
ImVec2_rr ImGui_CalcTextSize(const char* text, const char* text_end, bool hide_text_after_double_hash, float wrap_width) { return _rr(ImGui::CalcTextSize(text, text_end, hide_text_after_double_hash, wrap_width)); }
ImVec2_rr ImGui_GetMousePos() { return _rr(ImGui::GetMousePos()); }
ImVec2_rr ImGui_GetMousePosOnOpeningCurrentPopup() { return _rr(ImGui::GetMousePosOnOpeningCurrentPopup()); }
ImVec2_rr ImGui_GetMouseDragDelta(ImGuiMouseButton button, float lock_threshold) { return _rr(ImGui::GetMouseDragDelta(button, lock_threshold)); }
