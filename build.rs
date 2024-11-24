use std::{env, fs::File, path::PathBuf};

use gl_generator::{Api, DebugStructGenerator, Fallbacks, Profile, Registry, StructGenerator};

fn main() {
    let dest = PathBuf::from(&env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed=build.rs");

    let mut file = File::create(dest.join("gl_bindings.rs")).unwrap();

    if cfg!(feature = "debug_gl_structs") {
        Registry::new(Api::Gl, (4, 6), Profile::Core, Fallbacks::None, [])
            .write_bindings(DebugStructGenerator, &mut file)
            .unwrap();
    } else {
        Registry::new(Api::Gl, (4, 6), Profile::Core, Fallbacks::None, [])
            .write_bindings(StructGenerator, &mut file)
            .unwrap();
    }
}
