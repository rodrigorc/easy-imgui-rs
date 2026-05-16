// Non trivial  inline functions, make them non-inline
void ImGui_ImVector_vec2_push_back(ImVector<ImVec2> *vs, const ImVec2 *v) {
    vs->push_back(*v);
}
