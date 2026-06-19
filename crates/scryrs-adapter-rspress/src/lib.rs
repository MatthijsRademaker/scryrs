//! Optional Rspress adapter foundation.

use scryrs_types::FeatureDescriptor;

pub fn descriptor() -> FeatureDescriptor {
    FeatureDescriptor {
        id: "adapter-rspress",
        title: "scryrs-adapter-rspress",
        summary: "Rspress publishing surface foundation",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RspressRoute {
    pub path: String,
    pub source_markdown: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_marks_rspress_adapter() {
        assert_eq!(descriptor().id, "adapter-rspress");
    }
}
