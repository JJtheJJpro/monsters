cfg_select! {
    target_os = "linux" => {
        pub(super) mod linux;
        pub(super) use linux as os;
    }
    target_os = "macos" => {
        pub(super) mod macos;
        pub(super) use macos as os;
    }
    target_os = "windows" => {
        pub(super) mod windows;
        pub(super) use windows as os;
    }
}
