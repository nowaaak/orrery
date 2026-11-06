#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    MacOs,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86,
    X86_64,
    Aarch64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

impl Platform {
    pub fn current() -> Self {
        let os = match std::env::consts::OS {
            "windows" => Os::Windows,
            "macos" => Os::MacOs,
            _ => Os::Linux,
        };
        let arch = match std::env::consts::ARCH {
            "x86" => Arch::X86,
            "aarch64" => Arch::Aarch64,
            _ => Arch::X86_64,
        };
        Self { os, arch }
    }

    pub fn classpath_separator(&self) -> &'static str {
        match self.os {
            Os::Windows => ";",
            Os::MacOs | Os::Linux => ":",
        }
    }
}
