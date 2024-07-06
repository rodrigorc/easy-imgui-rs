# easy-imgui-filechooser

File-chooser widget for `easy-imgui`.

It is inspired by [ImGuiFileDialog ][1] (thanks!), but this is written in Rust instead of C++, and with fewer options.

This a very basic file-chooser widget. It does not provide the pop-up window, previews or any kind of validation. That is up to the user.

It pretends to be portable to any target where you can run `std` Rust and `easy-imgui`. This is the look for Linux:

![image](https://github.com/rodrigorc/easy-imgui-rs/assets/1128630/275230fd-cee7-446d-a5ab-646e333a3cdb)


[1]: https://github.com/aiekick/ImGuiFileDialog 
