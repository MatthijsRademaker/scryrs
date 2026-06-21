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

    #[test]
    fn descriptor_fields_are_non_empty() {
        let d = descriptor();
        assert!(!d.title.is_empty());
        assert!(!d.summary.is_empty());
    }

    #[test]
    fn rspress_route_stores_path_and_source() {
        let route = RspressRoute {
            path: "/docs/architecture".to_string(),
            source_markdown: "# Architecture".to_string(),
        };
        assert_eq!(route.path, "/docs/architecture");
        assert_eq!(route.source_markdown, "# Architecture");
    }

    #[test]
    fn rspress_route_supports_empty_source() {
        let route = RspressRoute {
            path: "/empty".to_string(),
            source_markdown: String::new(),
        };
        assert_eq!(route.path, "/empty");
        assert!(route.source_markdown.is_empty());
    }
}
