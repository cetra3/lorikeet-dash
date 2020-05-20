
extern crate includedir_codegen;

use includedir_codegen::Compression;

fn main() {
    includedir_codegen::start("FILES")
        .dir("front/dist", Compression::None)
        .build("data.rs")
        .unwrap();
}
