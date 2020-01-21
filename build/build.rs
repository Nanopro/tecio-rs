use std::env;

fn main() {
    #[cfg(any(test))]
    {
        println!("cargo:warning=Link dynamicly becouse of tests");
        println!(r"cargo:rustc-link-search=shared=C:\Program Files\Tecplot\Tecplot 360 EX 2018 R2\lib");
    }
    #[cfg(all(feature = "link_static"))]
    {
        //println!("cargo:rustc-link-search=native=tecio");
        //println!("cargo:rustc-link-lib=static=my_c_lib");
        //println!("cargo:rustc-env=RUST_BACKTRACE=1");

        use cmake::*;
        let mut config = cmake::Config::new("build/tecio-src");

        config.profile("Release").generator("Ninja");

        let mut lib_path = config.build();

        emit_std_cpp_link();
        println!("cargo:warning=Asked to build from source");
        println!("cargo:warning=Linking to {}", lib_path.display());
        println!("cargo:rustc-link-search=native={}", lib_path.display());
        println!("cargo:rustc-link-lib=static=tecio");
    }
    #[cfg(not(feature = "link_static"))]
    {
        println!("cargo:warning=Asked to link dynamicly");
        println!("cargo:rustc-link-lib=dylib=tecio");
    }

}

fn emit_std_cpp_link() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap();

    match (target_os.as_str(), target_env.as_str()) {
        ("linux", _) | ("windows", "gnu") => println!("cargo:rustc-link-lib=dylib=stdc++"),
        ("macos", _) => println!("cargo:rustc-link-lib=dylib=c++"),
        ("windows", _) => println!("cargo:rustc-link-lib=dylib=user32"),
        _ => {
            println!("cargo:warning=Failed to link with stdc++");
        }
    }
}
