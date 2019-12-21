


fn main() {
    #[cfg(all(feature = "link_static"))]
    {

        //println!("cargo:rustc-link-search=native=tecio");
        //println!("cargo:rustc-link-lib=static=my_c_lib");
        //println!("cargo:rustc-env=RUST_BACKTRACE=1");


        use cmake::*;
        let mut config = cmake::Config::new("build/tecio-src");


        config
            .profile("Release")
            .generator("Ninja");


        let mut lib_path = config.build();


        lib_path.push("lib");
        println!("cargo:warning=Asked to build from source");
        println!("cargo:rustc-link-search=native={}", lib_path.display());
        println!("cargo:rustc-link-lib=static=tecio");
    }
    #[cfg(feature = "link_dynamic")]
    {
        println!("cargo:warning=Asked to link dynamicly");
        println!("cargo:rustc-link-lib=dylib=tecio");
    }

}

