fn main() {
    
    //println!("cargo:rustc-link-search=native=tecio");
    //println!("cargo:rustc-link-lib=static=my_c_lib");
    //println!("cargo:rustc-env=RUST_BACKTRACE=1");
    println!("cargo:rustc-link-lib=dylib=tecio");
    println!(r"cargo:rustc-link-search=dylib=C:\Program Files\Tecplot\Tecplot 360 EX 2018 R2\lib");

}