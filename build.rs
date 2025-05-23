fn main() {
    #[cfg(feature = "c-tests")]
    {
        let mut b = cc::Build::new();

        b.compiler("clang");
        b.file("tests/c/interp.c");
        b.file("tests/c/true_peak.c");
        b.file("tests/c/history.c");
        b.file("tests/c/filter.c");
        b.file("tests/c/calc_gating_block.c");
        b.compile("ebur128");
    }

    #[cfg(feature = "bindgen")]
    {
        csbindgen::Builder::default()
            .input_extern_file("src/capi.rs")
            .input_extern_file("src/ebur128.rs")
            .input_extern_file("src/filter.rs")
            .input_extern_file("src/histogram_bins.rs")
            .input_extern_file("src/history.rs")
            .input_extern_file("src/interp.rs")
            .input_extern_file("src/lib.rs")
            .input_extern_file("src/true_peak.rs")
            .input_extern_file("src/utils.rs")
            .csharp_dll_name("ebur128")
            .csharp_class_name("EbuR128Native")
            .csharp_namespace("libebur128")
            .csharp_file_header("// @formatter:off\n#pragma warning disable format")
            .generate_csharp_file("../libebur128/EbuR128.g.cs")
            .unwrap();
    }
}
