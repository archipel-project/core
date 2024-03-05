use std::fmt::Display;

/// Represents the size of memory in bytes, kilobytes, megabytes, or gigabytes. This is useful for displaying memory usage.
pub enum MemorySize {
    Bytes(usize),
    KiloBytes(usize),
    MegaBytes(usize),
    GigaBytes(usize),
}

impl MemorySize {
    fn new(bytes: usize) -> Self {
        if bytes < 1024 {
            Self::Bytes(bytes)
        } else if bytes < 1024 * 1024 {
            Self::KiloBytes(bytes / 1024)
        } else if bytes < 1024 * 1024 * 1024 {
            Self::MegaBytes(bytes / (1024 * 1024))
        } else {
            Self::GigaBytes(bytes / (1024 * 1024 * 1024))
        }
    }
}

impl From<usize> for MemorySize {
    fn from(bytes: usize) -> Self {
        Self::new(bytes)
    }
}

impl Display for MemorySize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemorySize::Bytes(bytes) => write!(f, "{} bytes", bytes),
            MemorySize::KiloBytes(kb) => write!(f, "{} KB", kb),
            MemorySize::MegaBytes(mb) => write!(f, "{} MB", mb),
            MemorySize::GigaBytes(gb) => write!(f, "{} GB", gb),
        }
    }
}
