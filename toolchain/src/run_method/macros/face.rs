//! Registration-face macro ladder.
//! 注册面宏阶梯。
//!
//! The ladder's arms and their helpers live in the mounted pages; this page only
//! mounts them. The IDE-only field mirror that used to live here was retired in
//! step ⑤ of A2e: the new face spelling is plain Rust, so an editor completes it
//! natively and no `cfg(rust_analyzer)` splice is needed. The field vocabulary it
//! listed is documented where the author reads it — on the generated alias macros.
//! 阶梯的各臂与它们的助手住在挂载进来的页面里，本页只负责挂载。这里原来的"仅供编辑器的字段
//! 镜像"已在 A2e 第⑤步退役：新的面拼法是普通 Rust，编辑器原生即可补全，不再需要
//! `cfg(rust_analyzer)` 拼接。它当年列出的字段词表，如今写在作者看得到的地方——生成的别名宏上。

#[path = "face_external.rs"]
mod face_external;
#[path = "face_helpers.rs"]
mod face_helpers;
#[path = "face_objects.rs"]
mod face_objects;
#[path = "face_registration.rs"]
mod face_registration;
