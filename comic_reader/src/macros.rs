#[macro_export]
macro_rules! comic_filters {
    () => {{
        let dlg = rfd::FileDialog::new();

        #[cfg(feature = "rar")]
        let dlg = dlg
            .add_filter("Comic ZIP", &["cbz", "zip"])
            .add_filter("Comic RAR", &["cbr", "rar"]);

        #[cfg(not(feature = "rar"))]
        let dlg = dlg.add_filter("Comic Book Archive", &["cbz", "zip"]);

        dlg
    }};
}
